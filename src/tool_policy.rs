//! 统一工具策略：查询分类、候选工具过滤、系统提示、调用改写与执行前阻断。

use chrono::Local;
use regex::Regex;
use serde_json::Value;

use crate::tools::{
    ToolCapabilityGroup, ToolCostClass, ToolFreshness, ToolMetadata, ToolScope, ToolUseCase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    Weather,
    News,
    ExchangeRate,
    MarketQuote,
    SportsScore,
    TimeSensitiveCurrent,
    WebPageAnalysis,
    DirectExplanation,
    ExternalGitHubRepo,
    General,
}

#[derive(Debug, Clone)]
pub struct ToolPolicyDecision {
    pub allowed_tools: Vec<String>,
    pub system_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolCallPolicyResult {
    pub tool_name: String,
    pub args: Value,
    pub rewritten_from: Option<String>,
}

fn extract_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|word| word.starts_with("http://") || word.starts_with("https://"))
        .map(ToString::to_string)
}

fn is_github_repo_url(url: &str) -> bool {
    let trimmed = url.trim_end_matches('/');
    let Some(stripped) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
    else {
        return false;
    };
    let segments: Vec<&str> = stripped
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    segments.len() >= 2
}

fn contains_local_workspace_signal(input_lower: &str) -> bool {
    [
        "本地",
        "workspace",
        "当前仓库",
        "当前项目",
        "本项目",
        "this workspace",
        "local repo",
        "local repository",
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword))
}

fn contains_any(input_lower: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| input_lower.contains(keyword))
}

fn is_github_architecture_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "架构",
            "技术架构",
            "系统设计",
            "实现原理",
            "技术栈",
            "architecture",
            "system design",
            "technical architecture",
            "tech stack",
            "backend",
            "frontend",
            "database",
            "orchestration",
            "package.json",
            "cargo.toml",
            "readme",
            "源码",
            "source code",
            "repo structure",
        ],
    )
}

fn is_github_locator_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "开源地址",
            "仓库地址",
            "repo",
            "repository",
            "github",
            "homepage",
            "下载地址",
            "开源链接",
        ],
    )
}

fn is_direct_explanation_query(input_lower: &str) -> bool {
    let asks_for_explanation = contains_any(
        input_lower,
        &[
            "是什么",
            "做什么",
            "介绍一下",
            "介绍下",
            "核心功能",
            "主要功能",
            "产品",
            "服务",
            "what is",
            "what does",
            "describe",
            "overview",
            "core functions",
            "main features",
        ],
    );

    let asks_for_inspection = contains_any(
        input_lower,
        &[
            "列出",
            "ls",
            "目录",
            "文件",
            "readme",
            "package.json",
            "cargo.toml",
            "源码",
            "代码",
            "搜索",
            "search",
            "验证",
            "verify",
            "链接",
            "url",
            "http://",
            "https://",
        ],
    );

    asks_for_explanation && !asks_for_inspection
}

fn is_web_page_analysis_query(input_lower: &str) -> bool {
    let asks_for_web_analysis = contains_any(
        input_lower,
        &[
            "网站",
            "网页",
            "页面",
            "前端",
            "ui",
            "web ui",
            "landing page",
            "frontend",
            "web",
        ],
    ) && contains_any(
        input_lower,
        &["分析", "看看", "review", "analyze", "inspect"],
    );

    asks_for_web_analysis && !contains_local_workspace_signal(input_lower)
}

fn is_time_sensitive_current_query(input_lower: &str) -> bool {
    let recency = contains_any(
        input_lower,
        &[
            "今天", "今日", "最新", "最近", "当前", "刚刚", "today", "latest", "recent", "current",
            "breaking",
        ],
    );

    let domains = contains_any(
        input_lower,
        &[
            "新闻", "news", "天气", "weather", "头条", "热点", "股价", "汇率", "价格", "score",
            "比分",
        ],
    );

    recency && domains
}

fn is_weather_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "天气",
            "weather",
            "forecast",
            "气温",
            "降雨",
            "明天",
            "today weather",
            "tomorrow weather",
        ],
    )
}

fn is_news_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "新闻",
            "news",
            "头条",
            "热点",
            "快讯",
            "top stories",
            "headlines",
        ],
    )
}

fn is_exchange_rate_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "汇率",
            "exchange rate",
            "fx",
            "外汇",
            "美元兑",
            "人民币兑",
            "usd/cny",
            "eur/usd",
        ],
    )
}

fn is_market_quote_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "股价",
            "股票",
            "quote",
            "ticker",
            "btc",
            "bitcoin",
            "以太坊",
            "比特币",
            "纳指",
            "标普",
            "dow",
            "crypto price",
            "price of",
        ],
    )
}

fn is_sports_score_query(input_lower: &str) -> bool {
    contains_any(
        input_lower,
        &[
            "比分", "score", "赛果", "战报", "nba", "nfl", "mlb", "nhl", "英超", "epl",
        ],
    )
}

fn query_use_cases(kind: QueryKind) -> Vec<ToolUseCase> {
    match kind {
        QueryKind::Weather => vec![ToolUseCase::Weather, ToolUseCase::TimeSensitiveCurrent],
        QueryKind::News => vec![ToolUseCase::News, ToolUseCase::TimeSensitiveCurrent],
        QueryKind::ExchangeRate => {
            vec![ToolUseCase::ExchangeRate, ToolUseCase::TimeSensitiveCurrent]
        }
        QueryKind::MarketQuote => {
            vec![ToolUseCase::MarketQuote, ToolUseCase::TimeSensitiveCurrent]
        }
        QueryKind::SportsScore => {
            vec![ToolUseCase::SportsScore, ToolUseCase::TimeSensitiveCurrent]
        }
        QueryKind::TimeSensitiveCurrent => vec![ToolUseCase::TimeSensitiveCurrent],
        QueryKind::WebPageAnalysis => vec![ToolUseCase::DirectExplanation],
        QueryKind::DirectExplanation => vec![ToolUseCase::DirectExplanation],
        QueryKind::ExternalGitHubRepo => vec![ToolUseCase::ExternalGitHubRepo],
        QueryKind::General => Vec::new(),
    }
}

fn metadata_matches_use_case(metadata: &ToolMetadata, use_case: ToolUseCase) -> bool {
    !metadata.disallowed_use_cases.contains(&use_case)
        && (metadata.preferred_use_cases.is_empty()
            || metadata.preferred_use_cases.contains(&use_case))
}

fn metadata_supports_query(metadata: &ToolMetadata, kind: QueryKind) -> bool {
    let use_cases = query_use_cases(kind);
    if use_cases.is_empty() {
        return true;
    }

    let specialized_match = use_cases
        .iter()
        .copied()
        .any(|use_case| metadata_matches_use_case(metadata, use_case));

    if specialized_match {
        return true;
    }

    match kind {
        QueryKind::Weather
        | QueryKind::News
        | QueryKind::ExchangeRate
        | QueryKind::MarketQuote
        | QueryKind::SportsScore
        | QueryKind::TimeSensitiveCurrent => {
            matches!(metadata.scope, ToolScope::RemoteWeb | ToolScope::GitHub)
                && !metadata.requires_explicit_user_request
                && metadata.freshness != ToolFreshness::Static
        }
        QueryKind::ExternalGitHubRepo => {
            matches!(metadata.scope, ToolScope::GitHub | ToolScope::RemoteWeb)
        }
        QueryKind::WebPageAnalysis => {
            matches!(metadata.scope, ToolScope::RemoteWeb | ToolScope::GitHub)
                && !metadata.requires_explicit_user_request
        }
        QueryKind::DirectExplanation => {
            !matches!(
                metadata.scope,
                ToolScope::LocalWorkspace | ToolScope::System
            ) && !metadata.requires_explicit_user_request
        }
        QueryKind::General => true,
    }
}

fn preferred_capability_group(kind: QueryKind) -> Option<ToolCapabilityGroup> {
    match kind {
        QueryKind::Weather
        | QueryKind::News
        | QueryKind::ExchangeRate
        | QueryKind::MarketQuote
        | QueryKind::SportsScore
        | QueryKind::TimeSensitiveCurrent => Some(ToolCapabilityGroup::RealtimeData),
        QueryKind::WebPageAnalysis => Some(ToolCapabilityGroup::WebResearch),
        QueryKind::ExternalGitHubRepo => Some(ToolCapabilityGroup::RepositoryAnalysis),
        QueryKind::DirectExplanation => Some(ToolCapabilityGroup::DirectAnswer),
        QueryKind::General => None,
    }
}

fn cost_score(cost: ToolCostClass) -> u16 {
    match cost {
        ToolCostClass::Low => 0,
        ToolCostClass::Medium => 10,
        ToolCostClass::High => 20,
    }
}

fn metadata_rank_score(kind: QueryKind, metadata: &ToolMetadata) -> u16 {
    let mut score = metadata.preferred_rank as u16;
    if preferred_capability_group(kind).is_some_and(|group| metadata.capability_group == group) {
        score = score.saturating_sub(20);
    }
    if matches!(
        kind,
        QueryKind::Weather
            | QueryKind::News
            | QueryKind::ExchangeRate
            | QueryKind::MarketQuote
            | QueryKind::SportsScore
            | QueryKind::TimeSensitiveCurrent
    ) && metadata.freshness == ToolFreshness::Live
    {
        score = score.saturating_sub(10);
    }
    score
        + cost_score(metadata.overall_cost_class)
        + cost_score(metadata.latency_class)
        + cost_score(metadata.token_cost_class)
        + cost_score(metadata.api_cost_class)
}

fn parse_first_number(text: &str) -> Option<usize> {
    let re = Regex::new(r"(\d{1,2})").ok()?;
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
}

fn infer_weather_day(input_lower: &str) -> &'static str {
    if input_lower.contains("明天") || input_lower.contains("tomorrow") {
        "tomorrow"
    } else {
        "today"
    }
}

fn clean_location_text(text: &str) -> String {
    text.replace("今天天气", "")
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
        .trim()
        .trim_end_matches('的')
        .trim()
        .to_string()
}

fn infer_weather_location(text: &str) -> Option<String> {
    let location = clean_location_text(text);
    if location.is_empty() {
        None
    } else {
        Some(location)
    }
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
    let upper_re = Regex::new(r"\b([A-Z]{3})\b").ok()?;
    let uppercase_text = text.to_uppercase();
    let mut found_codes: Vec<String> = upper_re
        .captures_iter(&uppercase_text)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect();

    for (alias, code) in currency_aliases() {
        if text.to_lowercase().contains(alias) {
            found_codes.push(code.to_string());
        }
    }

    found_codes.dedup();
    if found_codes.len() >= 2 {
        return Some((found_codes[0].clone(), found_codes[1].clone()));
    }

    None
}

fn infer_market_symbols(text: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    let mappings = [
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
    ];

    let lower = text.to_lowercase();
    for (keyword, symbol) in mappings {
        if lower.contains(keyword) {
            symbols.push(symbol.to_string());
        }
    }

    if let Ok(re) = Regex::new(r"\b[A-Z]{1,5}(?:-[A-Z]{3})?\b") {
        for caps in re.captures_iter(text) {
            if let Some(symbol) = caps.get(0) {
                symbols.push(symbol.as_str().to_string());
            }
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

fn query_kind_system_hint(kind: QueryKind) -> Option<String> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    match kind {
        QueryKind::Weather => Some(format!(
            "This is a live weather request. Today's date is {today}. Prefer the weather tool and mention exact dates like {today} or tomorrow explicitly."
        )),
        QueryKind::News => Some(format!(
            "This is a live news request. Today's date is {today}. Prefer the news tool, prioritize very recent items, and mention exact publication dates."
        )),
        QueryKind::ExchangeRate => Some(format!(
            "This is a live exchange-rate request. Today's date is {today}. Prefer the exchange_rate tool and return exact currency pairs."
        )),
        QueryKind::MarketQuote => Some(format!(
            "This is a live market quote request. Today's date is {today}. Prefer the market_quote tool and mention the exact symbol and quote time when available."
        )),
        QueryKind::SportsScore => Some(format!(
            "This is a live sports-score request. Today's date is {today}. Prefer the sports_score tool and mention the exact league and game status."
        )),
        QueryKind::TimeSensitiveCurrent => Some(format!(
            "This is a time-sensitive current-information request. Today's date is {today}. Use fresh tools and prioritize results from {today}. Do not rely on stale memory or older summaries. If you answer with dates, mention exact dates explicitly."
        )),
        QueryKind::WebPageAnalysis => Some(
            "This request is about analyzing a website, web page, or frontend UI. Prefer remote web tools like search or browser. Do not inspect the local workspace with ls, cat, code_read, or shell unless the user explicitly asks about local files. If no URL or target site is provided, ask a short clarification question instead of probing the local workspace."
                .to_string(),
        ),
        QueryKind::ExternalGitHubRepo => Some(
            "This request is about an external GitHub repository. Prefer github_repo_inspect. Do not use local workspace tools like ls, cat, code_read, or shell unless the user explicitly asks about local files."
                .to_string(),
        ),
        QueryKind::DirectExplanation => Some(
            "This is a direct explanation request. Answer directly from the conversation and available context. Do not call ls, cat, code_read, code_grep, or shell unless the user explicitly asks to inspect files or verify details."
                .to_string(),
        ),
        QueryKind::General => None,
    }
}

pub fn classify_query(user_input: &str) -> QueryKind {
    let input_lower = user_input.to_lowercase();

    if is_weather_query(&input_lower) {
        return QueryKind::Weather;
    }
    if is_news_query(&input_lower) {
        return QueryKind::News;
    }
    if is_exchange_rate_query(&input_lower) {
        return QueryKind::ExchangeRate;
    }
    if is_market_quote_query(&input_lower) {
        return QueryKind::MarketQuote;
    }
    if is_sports_score_query(&input_lower) {
        return QueryKind::SportsScore;
    }
    if is_time_sensitive_current_query(&input_lower) {
        return QueryKind::TimeSensitiveCurrent;
    }
    if is_web_page_analysis_query(&input_lower) {
        return QueryKind::WebPageAnalysis;
    }
    if let Some(url) = extract_url(user_input) {
        if is_github_repo_url(&url)
            && !contains_local_workspace_signal(&input_lower)
            && (is_github_architecture_query(&input_lower) || is_github_locator_query(&input_lower))
        {
            return QueryKind::ExternalGitHubRepo;
        }
    }
    if is_direct_explanation_query(&input_lower) {
        return QueryKind::DirectExplanation;
    }

    QueryKind::General
}

pub fn refine_allowed_tools_for_input(
    user_input: &str,
    allowed_metadata: &[(String, ToolMetadata)],
) -> ToolPolicyDecision {
    let kind = classify_query(user_input);
    let use_cases = query_use_cases(kind);
    let exact_matches: Vec<String> = if use_cases.is_empty() {
        Vec::new()
    } else {
        allowed_metadata
            .iter()
            .filter(|(_, metadata)| {
                use_cases
                    .iter()
                    .copied()
                    .any(|use_case| metadata_matches_use_case(metadata, use_case))
            })
            .map(|(name, _)| name.clone())
            .collect()
    };

    let filtered: Vec<(String, ToolMetadata)> = allowed_metadata
        .iter()
        .filter(|(_, metadata)| metadata_supports_query(metadata, kind))
        .map(|(name, metadata)| (name.clone(), metadata.clone()))
        .collect();

    let allowed_tools = if !exact_matches.is_empty() {
        exact_matches
    } else if filtered.is_empty() {
        allowed_metadata
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    } else {
        let mut ranked = filtered;
        ranked.sort_by_key(|(_, metadata)| metadata_rank_score(kind, metadata));
        ranked.into_iter().take(5).map(|(name, _)| name).collect()
    };

    ToolPolicyDecision {
        allowed_tools,
        system_hint: query_kind_system_hint(kind),
    }
}

pub fn should_use_long_term_memory(user_input: &str) -> bool {
    !matches!(
        classify_query(user_input),
        QueryKind::Weather
            | QueryKind::News
            | QueryKind::ExchangeRate
            | QueryKind::MarketQuote
            | QueryKind::SportsScore
            | QueryKind::WebPageAnalysis
            | QueryKind::TimeSensitiveCurrent
    )
}

fn infer_search_like_query<'a>(tool: &str, args: &'a Value) -> Option<&'a str> {
    if tool == "deep_search" {
        args.get("topic").and_then(|value| value.as_str())
    } else if tool == "search" {
        args.get("url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.starts_with("http://") && !value.starts_with("https://"))
    } else {
        None
    }
}

pub fn rewrite_tool_call(user_input: &str, tool: &str, args: &Value) -> ToolCallPolicyResult {
    if tool == "search" {
        if let Some(url) = args.get("url").and_then(|value| value.as_str()) {
            if is_github_repo_url(url) {
                return ToolCallPolicyResult {
                    tool_name: "github_repo_inspect".to_string(),
                    args: serde_json::json!({ "url": url }),
                    rewritten_from: Some(tool.to_string()),
                };
            }
        }
    }

    let query_text = infer_search_like_query(tool, args).unwrap_or(user_input);
    let lower = query_text.to_lowercase();
    let kind = classify_query(user_input);

    match kind {
        QueryKind::Weather if tool != "weather" => {
            if let Some(location) = infer_weather_location(query_text) {
                return ToolCallPolicyResult {
                    tool_name: "weather".to_string(),
                    args: serde_json::json!({
                        "location": location,
                        "day": infer_weather_day(&lower),
                    }),
                    rewritten_from: Some(tool.to_string()),
                };
            }
        }
        QueryKind::News if tool != "news" => {
            return ToolCallPolicyResult {
                tool_name: "news".to_string(),
                args: serde_json::json!({
                    "query": infer_news_query(query_text),
                    "limit": infer_news_limit(query_text),
                }),
                rewritten_from: Some(tool.to_string()),
            };
        }
        QueryKind::ExchangeRate if tool != "exchange_rate" => {
            if let Some((base, quote)) = infer_exchange_pair(query_text) {
                return ToolCallPolicyResult {
                    tool_name: "exchange_rate".to_string(),
                    args: serde_json::json!({
                        "base": base,
                        "quote": quote,
                    }),
                    rewritten_from: Some(tool.to_string()),
                };
            }
        }
        QueryKind::MarketQuote if tool != "market_quote" => {
            let symbols = infer_market_symbols(query_text);
            if !symbols.is_empty() {
                return ToolCallPolicyResult {
                    tool_name: "market_quote".to_string(),
                    args: serde_json::json!({ "symbols": symbols }),
                    rewritten_from: Some(tool.to_string()),
                };
            }
        }
        QueryKind::SportsScore if tool != "sports_score" => {
            if let Some(league) = infer_sports_league(query_text) {
                return ToolCallPolicyResult {
                    tool_name: "sports_score".to_string(),
                    args: serde_json::json!({
                        "league": league,
                    }),
                    rewritten_from: Some(tool.to_string()),
                };
            }
        }
        _ => {}
    }

    ToolCallPolicyResult {
        tool_name: tool.to_string(),
        args: args.clone(),
        rewritten_from: None,
    }
}

pub fn guard_tool_call(
    user_input: &str,
    tool_name: &str,
    tool_metadata: Option<&ToolMetadata>,
    _args: &Value,
) -> Result<(), String> {
    let kind = classify_query(user_input);
    let Some(metadata) = tool_metadata else {
        return Ok(());
    };

    if matches!(
        kind,
        QueryKind::Weather
            | QueryKind::News
            | QueryKind::ExchangeRate
            | QueryKind::MarketQuote
            | QueryKind::SportsScore
            | QueryKind::TimeSensitiveCurrent
    ) && matches!(
        metadata.scope,
        ToolScope::LocalWorkspace | ToolScope::System
    ) {
        return Err(format!(
            "This is a current-information request; use fresh remote tools instead of {}.",
            tool_name
        ));
    }

    if kind == QueryKind::ExternalGitHubRepo
        && matches!(
            metadata.scope,
            ToolScope::LocalWorkspace | ToolScope::System
        )
    {
        return Err(format!(
            "This request targets an external GitHub repository; do not use local tool {}.",
            tool_name
        ));
    }

    if kind == QueryKind::WebPageAnalysis
        && matches!(
            metadata.scope,
            ToolScope::LocalWorkspace | ToolScope::System
        )
    {
        return Err(format!(
            "This request targets a website or web page; do not use local tool {}.",
            tool_name
        ));
    }

    if kind == QueryKind::DirectExplanation
        && (matches!(
            metadata.scope,
            ToolScope::LocalWorkspace | ToolScope::System
        ) || metadata.requires_explicit_user_request)
    {
        return Err(format!(
            "This is a direct explanation request; answer directly instead of using {}.",
            tool_name
        ));
    }

    if metadata.requires_explicit_user_request
        && matches!(
            kind,
            QueryKind::Weather
                | QueryKind::News
                | QueryKind::ExchangeRate
                | QueryKind::MarketQuote
                | QueryKind::SportsScore
                | QueryKind::WebPageAnalysis
                | QueryKind::TimeSensitiveCurrent
                | QueryKind::ExternalGitHubRepo
        )
    {
        return Err(format!(
            "Tool {} requires explicit inspection/edit intent and should not be used here.",
            tool_name
        ));
    }

    let use_cases = query_use_cases(kind);
    if use_cases
        .iter()
        .any(|use_case| metadata.disallowed_use_cases.contains(use_case))
    {
        return Err(format!(
            "Tool {} is disallowed for this query type; choose a more suitable tool.",
            tool_name
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolCriticMode, ToolIntent, ToolOutputShape, ToolRisk};

    fn local_file_tool() -> ToolMetadata {
        ToolMetadata::new(ToolScope::LocalWorkspace, vec![ToolIntent::ReadFile])
            .with_risk(ToolRisk::Low)
            .with_output_shape(ToolOutputShape::StructuredJson)
            .with_preferred_use_cases(vec![ToolUseCase::LocalWorkspaceInspection])
            .with_disallowed_use_cases(vec![
                ToolUseCase::DirectExplanation,
                ToolUseCase::TimeSensitiveCurrent,
                ToolUseCase::ExternalGitHubRepo,
            ])
            .with_requires_explicit_user_request(true)
            .with_critic_mode(ToolCriticMode::Skip)
    }

    fn github_tool() -> ToolMetadata {
        ToolMetadata::new(ToolScope::GitHub, vec![ToolIntent::InspectRepository])
            .with_freshness(ToolFreshness::BestEffort)
            .with_output_shape(ToolOutputShape::StructuredJson)
            .with_preferred_use_cases(vec![ToolUseCase::ExternalGitHubRepo])
    }

    fn news_tool() -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::RemoteWeb,
            vec![ToolIntent::FetchWebPage, ToolIntent::Research],
        )
        .with_freshness(ToolFreshness::Live)
        .with_preferred_use_cases(vec![ToolUseCase::News, ToolUseCase::TimeSensitiveCurrent])
    }

    #[test]
    fn test_classify_external_github_query() {
        assert_eq!(
            classify_query("分析 https://github.com/paperclipai/paperclip 的技术架构"),
            QueryKind::ExternalGitHubRepo
        );
    }

    #[test]
    fn test_classify_time_sensitive_current_query() {
        assert_eq!(classify_query("今天有什么新闻，推荐5条"), QueryKind::News);
    }

    #[test]
    fn test_classify_web_page_analysis_query() {
        assert_eq!(
            classify_query("分析这个 web 页面"),
            QueryKind::WebPageAnalysis
        );
    }

    #[test]
    fn test_refine_for_direct_explanation() {
        let allowed = vec![
            ("ls".to_string(), local_file_tool()),
            ("github_repo_inspect".to_string(), github_tool()),
        ];
        let decision = refine_allowed_tools_for_input("这个产品是什么？", &allowed);
        assert_eq!(
            decision.allowed_tools,
            vec!["github_repo_inspect".to_string()]
        );
    }

    #[test]
    fn test_refine_for_news_prefers_specialized_tool() {
        let allowed = vec![
            ("ls".to_string(), local_file_tool()),
            ("news".to_string(), news_tool()),
            ("github_repo_inspect".to_string(), github_tool()),
        ];
        let decision = refine_allowed_tools_for_input("今天有什么新闻，推荐5条", &allowed);
        assert_eq!(decision.allowed_tools, vec!["news".to_string()]);
    }

    #[test]
    fn test_rewrite_search_to_github_repo_inspect() {
        let result = rewrite_tool_call(
            "分析这个仓库架构",
            "search",
            &serde_json::json!({"url": "https://github.com/paperclipai/paperclip"}),
        );
        assert_eq!(result.tool_name, "github_repo_inspect");
        assert_eq!(result.rewritten_from.as_deref(), Some("search"));
    }

    #[test]
    fn test_guard_blocks_local_tool_for_direct_explanation() {
        let err = guard_tool_call(
            "这个服务是做什么的？",
            "ls",
            Some(&local_file_tool()),
            &serde_json::json!({}),
        )
        .unwrap_err();
        assert!(err.contains("direct explanation request"));
    }

    #[test]
    fn test_time_sensitive_query_disables_long_term_memory() {
        assert!(!should_use_long_term_memory("今天有什么新闻，推荐5条"));
        assert!(should_use_long_term_memory("介绍一下这个产品"));
    }

    #[test]
    fn test_rewrite_weather_to_specialized_tool() {
        let result = rewrite_tool_call(
            "吉隆坡明天天气",
            "deep_search",
            &serde_json::json!({"topic": "吉隆坡明天天气"}),
        );
        assert_eq!(result.tool_name, "weather");
        assert_eq!(result.args["location"], "吉隆坡");
        assert_eq!(result.args["day"], "tomorrow");
    }

    #[test]
    fn test_rewrite_news_to_specialized_tool() {
        let result = rewrite_tool_call(
            "今天有什么新闻，推荐5条",
            "search",
            &serde_json::json!({"url": "今天有什么新闻，推荐5条"}),
        );
        assert_eq!(result.tool_name, "news");
        assert_eq!(result.args["limit"], 5);
    }

    #[test]
    fn test_rewrite_exchange_to_specialized_tool() {
        let result = rewrite_tool_call(
            "美元兑人民币汇率",
            "deep_search",
            &serde_json::json!({"topic": "美元兑人民币汇率"}),
        );
        assert_eq!(result.tool_name, "exchange_rate");
        assert_eq!(result.args["base"], "USD");
        assert_eq!(result.args["quote"], "CNY");
    }

    #[test]
    fn test_rewrite_market_quote_to_specialized_tool() {
        let result = rewrite_tool_call(
            "比特币现在价格",
            "deep_search",
            &serde_json::json!({"topic": "比特币现在价格"}),
        );
        assert_eq!(result.tool_name, "market_quote");
        assert_eq!(result.args["symbols"][0], "BTC-USD");
    }

    #[test]
    fn test_rewrite_sports_score_to_specialized_tool() {
        let result = rewrite_tool_call(
            "今天 NBA 比分",
            "deep_search",
            &serde_json::json!({"topic": "今天 NBA 比分"}),
        );
        assert_eq!(result.tool_name, "sports_score");
        assert_eq!(result.args["league"], "nba");
    }
}
