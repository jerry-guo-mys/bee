use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::tools::output;
use crate::tools::{
    Tool, ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode,
    ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};

pub struct WeatherTool {
    client: Client,
    timeout_secs: u64,
}

impl WeatherTool {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .user_agent("Bee/1.0")
                .build()
                .unwrap_or_default(),
            timeout_secs,
        }
    }

    fn normalize_day(day: &str) -> &str {
        match day.trim().to_lowercase().as_str() {
            "today" | "今天" | "今日" => "today",
            "tomorrow" | "明天" => "tomorrow",
            _ => "today",
        }
    }

    fn infer_location_from_text(text: &str) -> Option<String> {
        let mut location = text
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
            .trim()
            .to_string();
        if location.ends_with('的') {
            location.pop();
        }
        let location = Self::sanitize_location(&location);
        if location.is_empty() {
            None
        } else {
            Some(location)
        }
    }

    fn sanitize_location(location: &str) -> String {
        location
            .trim()
            .trim_matches(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '？' | '?'
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

    fn day_label(day: &str) -> &'static str {
        if Self::normalize_day(day) == "tomorrow" {
            "明天"
        } else {
            "今天"
        }
    }

    fn build_brief(location: &str, day: &str, current: &Value, forecast: &Value) -> String {
        let current_temp = current
            .get("temp_C")
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty());
        let desc = current
            .get("weatherDesc")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty());
        let max_temp = forecast.get("maxtempC").and_then(|v| v.as_str());
        let min_temp = forecast.get("mintempC").and_then(|v| v.as_str());

        let mut parts = vec![format!("{}{}天气", location, Self::day_label(day))];
        if let Some(desc) = desc {
            parts.push(desc.to_string());
        }
        if let Some(current_temp) = current_temp {
            parts.push(format!("当前约{}°C", current_temp));
        }
        if let (Some(min_temp), Some(max_temp)) = (min_temp, max_temp) {
            parts.push(format!("{}~{}°C", min_temp, max_temp));
        }
        parts.join("，")
    }

    async fn fetch_weather(&self, location: &str, day: &str) -> Result<String, String> {
        let mut url = reqwest::Url::parse("https://wttr.in/").map_err(|e| e.to_string())?;
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| "Failed to build weather URL".to_string())?;
            segments.push(location);
        }
        url.query_pairs_mut().append_pair("format", "j1");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if response.status().is_success() {
            let json: Value = response
                .json()
                .await
                .map_err(|e| format!("JSON parse failed: {}", e))?;

            let index = if Self::normalize_day(day) == "tomorrow" {
                1usize
            } else {
                0usize
            };

            let current = json["current_condition"]
                .as_array()
                .and_then(|arr| arr.first())
                .cloned()
                .unwrap_or(Value::Null);
            let selected = json["weather"]
                .as_array()
                .and_then(|arr| arr.get(index))
                .cloned()
                .or_else(|| {
                    json["weather"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .cloned()
                })
                .unwrap_or(Value::Null);

            let day_key = if index == 1 { "tomorrow" } else { "today" };
            let brief = Self::build_brief(location, day_key, &current, &selected);
            let summary = format!("{}天气：{}", location, Self::day_label(day_key));

            return output::structured(
                self.name(),
                summary,
                true,
                serde_json::json!({
                    "location": location,
                    "day": day_key,
                    "brief": brief,
                    "current_condition": current,
                    "forecast": selected,
                    "source": "wttr.in"
                }),
            );
        }

        output::structured(
            self.name(),
            format!("Weather fetch failed for {}", location),
            false,
            serde_json::json!({
                "location": location,
                "day": Self::normalize_day(day),
                "status_code": response.status().as_u16(),
                "source": "wttr.in",
                "recoverable": true
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_location_trims_punctuation() {
        assert_eq!(WeatherTool::sanitize_location("吉隆坡？"), "吉隆坡");
        assert_eq!(
            WeatherTool::sanitize_location("  Kuala Lumpur?! "),
            "Kuala Lumpur"
        );
    }

    #[test]
    fn test_infer_location_from_text_strips_weather_noise() {
        assert_eq!(
            WeatherTool::infer_location_from_text("吉隆坡明天天气怎么样？").as_deref(),
            Some("吉隆坡")
        );
    }
}

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "weather"
    }

    fn description(&self) -> &str {
        "Get live weather for a location, including today or tomorrow forecast. Args: {\"location\": \"Kuala Lumpur\", \"day\": \"today|tomorrow\"}"
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::RemoteWeb,
            vec![ToolIntent::FetchWebPage, ToolIntent::Research],
        )
        .with_risk(ToolRisk::Low)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_freshness(ToolFreshness::Live)
        .with_preferred_use_cases(vec![
            ToolUseCase::Weather,
            ToolUseCase::TimeSensitiveCurrent,
        ])
        .with_capability(
            ToolCapabilityGroup::RealtimeData,
            ToolCapabilitySubgroup::Weather,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
        )
        .with_preferred_rank(1)
        .with_critic_mode(ToolCriticMode::Skip)
    }

    fn timeout_secs(&self) -> Option<u64> {
        Some(self.timeout_secs)
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City or location name"
                },
                "day": {
                    "type": "string",
                    "description": "today or tomorrow",
                    "default": "today"
                }
            },
            "required": ["location"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let location = args
            .get("location")
            .and_then(|v| v.as_str())
            .map(Self::sanitize_location)
            .or_else(|| {
                args.get("topic")
                    .and_then(|v| v.as_str())
                    .and_then(Self::infer_location_from_text)
            })
            .or_else(|| {
                args.get("query")
                    .and_then(|v| v.as_str())
                    .and_then(Self::infer_location_from_text)
            })
            .unwrap_or_default();
        let day = args.get("day").and_then(|v| v.as_str()).unwrap_or("today");

        if location.trim().is_empty() {
            return Err("Missing location".to_string());
        }

        self.fetch_weather(&location, day).await
    }
}
