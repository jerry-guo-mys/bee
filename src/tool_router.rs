//! 确定性工具路由：对高确定性请求直接分发到专用工具，减少 Planner 负担。

use serde_json::Value;

use crate::core::AgentError;
use crate::memory::{Message, Role};
use crate::react::{ContextManager, ReactEvent, ReactResult};
use crate::tool_policy::classify_query;
use crate::tool_policy::QueryKind;
use crate::tools::ToolExecutor;

#[derive(Debug, Clone)]
pub struct DeterministicToolRoute {
    pub tool_name: String,
    pub args: Value,
    pub reason: String,
}

fn parse_first_number(text: &str) -> Option<usize> {
    let mut digits = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            break;
        }
    }
    digits.parse().ok()
}

fn infer_weather_day(input_lower: &str) -> &'static str {
    if input_lower.contains("明天") || input_lower.contains("tomorrow") {
        "tomorrow"
    } else {
        "today"
    }
}

fn clean_location_text(text: &str) -> String {
    let mut cleaned = text
        .replace("今天天气", "")
        .replace("明天天气", "")
        .replace("天气", "")
        .replace("weather", "")
        .replace("forecast", "")
        .replace("today", "")
        .replace("tomorrow", "")
        .replace("今日", "")
        .replace("今天", "")
        .replace("明天", "")
        .replace("怎么样", "")
        .replace("如何", "")
        .replace("查询", "")
        .replace("推荐", "")
        .to_string();

    for marker in ["？", "?", "，", ",", "。", ".", "；", ";", "\n", "给出", "请", "并且", "并", "顺便"] {
        if let Some(idx) = cleaned.find(marker) {
            cleaned.truncate(idx);
            break;
        }
    }

    cleaned
        .trim()
        .trim_matches(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '？'
                        | '?'
                        | '。'
                        | '.'
                        | '！'
                        | '!'
                        | '，'
                        | ','
                        | '、'
                        | '；'
                        | ';'
                        | '：'
                        | ':'
                        | '的'
                )
        })
        .trim()
        .to_string()
}

fn infer_news_query(text: &str) -> Option<String> {
    let cleaned = text
        .replace("今天", "")
        .replace("今日", "")
        .replace("最新", "")
        .replace("最近", "")
        .replace("新闻", "")
        .replace("头条", "")
        .replace("热点", "")
        .replace("推荐", "")
        .replace("news", "")
        .replace("headlines", "")
        .replace("top stories", "")
        .trim()
        .to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn infer_news_limit(text: &str) -> usize {
    parse_first_number(text).unwrap_or(5).clamp(1, 10)
}

fn currency_aliases() -> [(&'static str, &'static str); 13] {
    [
        ("美元", "USD"),
        ("人民币", "CNY"),
        ("欧元", "EUR"),
        ("日元", "JPY"),
        ("港币", "HKD"),
        ("英镑", "GBP"),
        ("马币", "MYR"),
        ("令吉", "MYR"),
        ("usd", "USD"),
        ("cny", "CNY"),
        ("eur", "EUR"),
        ("jpy", "JPY"),
        ("hkd", "HKD"),
    ]
}

fn infer_exchange_pair(text: &str) -> Option<(String, String)> {
    let uppercase_text = text.to_uppercase();
    let mut found_codes = Vec::new();
    for token in uppercase_text.split(|ch: char| !ch.is_ascii_alphabetic()) {
        if token.len() == 3 {
            found_codes.push(token.to_string());
        }
    }
    let lower = text.to_lowercase();
    for (alias, code) in currency_aliases() {
        if lower.contains(alias) {
            found_codes.push(code.to_string());
        }
    }
    found_codes.dedup();
    if found_codes.len() >= 2 {
        Some((found_codes[0].clone(), found_codes[1].clone()))
    } else {
        None
    }
}

fn infer_market_symbols(text: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    let lower = text.to_lowercase();
    for (keyword, symbol) in [
        ("比特币", "BTC-USD"),
        ("bitcoin", "BTC-USD"),
        ("btc", "BTC-USD"),
        ("以太坊", "ETH-USD"),
        ("ethereum", "ETH-USD"),
        ("eth", "ETH-USD"),
        ("纳指", "^IXIC"),
        ("nasdaq", "^IXIC"),
        ("标普", "^GSPC"),
        ("sp500", "^GSPC"),
        ("dow", "^DJI"),
        ("道指", "^DJI"),
        ("gold", "GC=F"),
        ("黄金", "GC=F"),
    ] {
        if lower.contains(keyword) {
            symbols.push(symbol.to_string());
        }
    }
    for token in text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '^') {
        if !token.is_empty()
            && token
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-' || ch == '^')
            && (1..=8).contains(&token.len())
        {
            symbols.push(token.to_string());
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

fn infer_sports_league(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    for (keyword, league) in [
        ("nba", "nba"),
        ("nfl", "nfl"),
        ("mlb", "mlb"),
        ("nhl", "nhl"),
        ("英超", "epl"),
        ("epl", "epl"),
    ] {
        if lower.contains(keyword) {
            return Some(league);
        }
    }
    None
}

pub fn deterministic_route(
    user_input: &str,
    allowed_tools: Option<&[String]>,
) -> Option<DeterministicToolRoute> {
    let is_allowed = |tool_name: &str| {
        allowed_tools
            .map(|tools| tools.iter().any(|tool| tool == tool_name))
            .unwrap_or(true)
    };
    let input_lower = user_input.to_lowercase();

    match classify_query(user_input) {
        QueryKind::Weather if is_allowed("weather") => {
            let location = clean_location_text(user_input);
            if !location.is_empty() {
                return Some(DeterministicToolRoute {
                    tool_name: "weather".to_string(),
                    args: serde_json::json!({
                        "location": location,
                        "day": infer_weather_day(&input_lower),
                    }),
                    reason: "weather specialized tool".to_string(),
                });
            }
        }
        QueryKind::News if is_allowed("news") => {
            return Some(DeterministicToolRoute {
                tool_name: "news".to_string(),
                args: serde_json::json!({
                    "query": infer_news_query(user_input),
                    "limit": infer_news_limit(user_input),
                }),
                reason: "news specialized tool".to_string(),
            });
        }
        QueryKind::ExchangeRate if is_allowed("exchange_rate") => {
            if let Some((base, quote)) = infer_exchange_pair(user_input) {
                return Some(DeterministicToolRoute {
                    tool_name: "exchange_rate".to_string(),
                    args: serde_json::json!({ "base": base, "quote": quote }),
                    reason: "exchange-rate specialized tool".to_string(),
                });
            }
        }
        QueryKind::MarketQuote if is_allowed("market_quote") => {
            let symbols = infer_market_symbols(user_input);
            if !symbols.is_empty() {
                return Some(DeterministicToolRoute {
                    tool_name: "market_quote".to_string(),
                    args: serde_json::json!({ "symbols": symbols }),
                    reason: "market-quote specialized tool".to_string(),
                });
            }
        }
        QueryKind::SportsScore if is_allowed("sports_score") => {
            if let Some(league) = infer_sports_league(user_input) {
                return Some(DeterministicToolRoute {
                    tool_name: "sports_score".to_string(),
                    args: serde_json::json!({ "league": league }),
                    reason: "sports-score specialized tool".to_string(),
                });
            }
        }
        _ => {}
    }

    None
}

fn render_direct_response(tool: &str, raw_output: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(raw_output) else {
        return raw_output.trim().to_string();
    };
    let data = value.get("data").cloned().unwrap_or(Value::Null);
    match tool {
        "weather" => {
            if let Some(brief) = data.get("brief").and_then(Value::as_str) {
                if !brief.trim().is_empty() {
                    return brief.to_string();
                }
            }
            let location = data.get("location").and_then(Value::as_str).unwrap_or("");
            let day = data.get("day").and_then(Value::as_str).unwrap_or("today");
            let day_label = if day == "tomorrow" { "明天" } else { "今天" };
            let desc = data
                .get("current_condition")
                .and_then(|value| value.get("weatherDesc"))
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(|value| value.get("value"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let current_temp = data
                .get("current_condition")
                .and_then(|value| value.get("temp_C"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let min_temp = data
                .get("forecast")
                .and_then(|value| value.get("mintempC"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let max_temp = data
                .get("forecast")
                .and_then(|value| value.get("maxtempC"))
                .and_then(Value::as_str)
                .unwrap_or("");

            let mut parts = vec![format!("{location}{day_label}天气")];
            if !desc.is_empty() {
                parts.push(desc.to_string());
            }
            if !current_temp.is_empty() {
                parts.push(format!("当前约{}°C", current_temp));
            }
            if !min_temp.is_empty() && !max_temp.is_empty() {
                parts.push(format!("{}~{}°C", min_temp, max_temp));
            }
            if parts.len() == 1 {
                value
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("天气已获取")
                    .to_string()
            } else {
                parts.join("，")
            }
        }
        "news" => {
            let items = data
                .get("items")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut lines = Vec::new();
            for (index, item) in items.iter().take(5).enumerate() {
                let title = item.get("title").and_then(Value::as_str).unwrap_or("");
                let date = item.get("pub_date").and_then(Value::as_str).unwrap_or("");
                if !title.is_empty() {
                    if date.is_empty() {
                        lines.push(format!("{}. {}", index + 1, title));
                    } else {
                        lines.push(format!("{}. {} ({})", index + 1, title, date));
                    }
                }
            }
            if lines.is_empty() {
                value
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("No headlines found")
                    .to_string()
            } else {
                lines.join("\n")
            }
        }
        "exchange_rate" => format!(
            "{} {} = {} {}",
            data.get("amount").and_then(Value::as_f64).unwrap_or(1.0),
            data.get("base").and_then(Value::as_str).unwrap_or(""),
            data.get("converted_amount")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            data.get("quote").and_then(Value::as_str).unwrap_or(""),
        ),
        "market_quote" => {
            let quotes = data
                .get("quotes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let lines = quotes
                .iter()
                .take(5)
                .map(|quote| {
                    let symbol = quote.get("symbol").and_then(Value::as_str).unwrap_or("");
                    let price = quote.get("market_price").cloned().unwrap_or(Value::Null);
                    let currency = quote.get("currency").and_then(Value::as_str).unwrap_or("");
                    format!("{symbol}: {price} {currency}")
                })
                .collect::<Vec<_>>();
            if lines.is_empty() {
                value
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("No quotes found")
                    .to_string()
            } else {
                lines.join("\n")
            }
        }
        "sports_score" => {
            let games = data
                .get("games")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let lines = games
                .iter()
                .take(5)
                .map(|game| {
                    let name = game.get("short_name").and_then(Value::as_str).unwrap_or("");
                    let status = game.get("detail").and_then(Value::as_str).unwrap_or("");
                    format!("{name}: {status}")
                })
                .collect::<Vec<_>>();
            if lines.is_empty() {
                value
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("No game results found")
                    .to_string()
            } else {
                lines.join("\n")
            }
        }
        _ => value
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or(raw_output)
            .to_string(),
    }
}

pub async fn execute_direct_route(
    executor: &ToolExecutor,
    context: &mut ContextManager,
    user_input: &str,
    route: &DeterministicToolRoute,
    event_tx: Option<&tokio::sync::mpsc::UnboundedSender<ReactEvent>>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<ReactResult, AgentError> {
    if let Some(tx) = event_tx {
        tx.send(ReactEvent::ToolCall {
            tool: route.tool_name.clone(),
            args: route.args.clone(),
        })
        .ok();
    }
    let raw_output = executor
        .execute_cancellable(&route.tool_name, route.args.clone(), cancel_token)
        .await?;
    if let Some(tx) = event_tx {
        let preview = raw_output.chars().take(200).collect::<String>();
        tx.send(ReactEvent::Observation {
            tool: route.tool_name.clone(),
            preview,
        })
        .ok();
    }
    let response = render_direct_response(&route.tool_name, &raw_output);
    if let Some(tx) = event_tx {
        tx.send(ReactEvent::MessageChunk {
            text: response.clone(),
        })
        .ok();
        tx.send(ReactEvent::MessageDone).ok();
    }
    context.push_message(Message::user(user_input));
    context.push_message(Message {
        role: Role::Tool,
        content: format!("Tool call: {} | Result: {}", route.tool_name, raw_output),
    });
    context.push_message(Message::assistant(response.clone()));
    Ok(ReactResult {
        response,
        messages: context.messages().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_route_weather() {
        let route = deterministic_route("吉隆坡明天天气", None).unwrap();
        assert_eq!(route.tool_name, "weather");
        assert_eq!(route.args["day"], "tomorrow");
    }

    #[test]
    fn test_direct_route_news() {
        let route = deterministic_route("今天有什么新闻，推荐5条", None).unwrap();
        assert_eq!(route.tool_name, "news");
        assert_eq!(route.args["limit"], 5);
    }

    #[test]
    fn test_direct_route_skips_when_tool_not_allowed() {
        assert!(
            deterministic_route("今天有什么新闻，推荐5条", Some(&["search".to_string()])).is_none()
        );
    }

    #[test]
    fn test_clean_location_text_ignores_followup_style_request() {
        assert_eq!(
            clean_location_text("吉隆坡今天天气如何？给出更详细一点的回答，语气温情一点"),
            "吉隆坡"
        );
    }

    #[test]
    fn test_direct_route_weather_ignores_followup_style_request() {
        let route = deterministic_route(
            "吉隆坡今天天气如何？给出更详细一点的回答，语气温情一点",
            None,
        )
        .unwrap();
        assert_eq!(route.tool_name, "weather");
        assert_eq!(route.args["location"], "吉隆坡");
        assert_eq!(route.args["day"], "today");
    }

    #[test]
    fn test_render_direct_response_weather_prefers_brief() {
        let raw = serde_json::json!({
            "tool": "weather",
            "summary": "吉隆坡天气：今天",
            "sufficient_to_answer": true,
            "data": {
                "location": "吉隆坡",
                "day": "today",
                "brief": "吉隆坡今天天气，晴，当前约24°C，22~32°C"
            }
        })
        .to_string();
        assert_eq!(
            render_direct_response("weather", &raw),
            "吉隆坡今天天气，晴，当前约24°C，22~32°C"
        );
    }
}
