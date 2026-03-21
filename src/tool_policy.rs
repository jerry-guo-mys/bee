//! 统一工具策略：查询分类、候选工具过滤、系统提示、调用改写与执行前阻断。

use chrono::Local;
use serde_json::Value;

use crate::tools::{ToolIntent, ToolMetadata, ToolScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    TimeSensitiveCurrent,
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

fn is_github_architecture_query(input_lower: &str) -> bool {
    [
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
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword))
}

fn is_github_locator_query(input_lower: &str) -> bool {
    [
        "开源地址",
        "仓库地址",
        "repo",
        "repository",
        "github",
        "homepage",
        "下载地址",
        "开源链接",
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword))
}

fn is_direct_explanation_query(input_lower: &str) -> bool {
    let asks_for_explanation = [
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
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword));

    let asks_for_inspection = [
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
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword));

    asks_for_explanation && !asks_for_inspection
}

fn is_time_sensitive_current_query(input_lower: &str) -> bool {
    let recency = [
        "今天", "今日", "最新", "最近", "当前", "刚刚", "today", "latest", "recent", "current",
        "breaking",
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword));

    let domains = [
        "新闻", "news", "天气", "weather", "头条", "热点", "股价", "汇率", "价格", "score", "比分",
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword));

    recency && domains
}

fn is_weather_query(input_lower: &str) -> bool {
    [
        "天气",
        "weather",
        "forecast",
        "气温",
        "降雨",
        "明天",
        "today weather",
        "tomorrow weather",
    ]
    .iter()
    .any(|keyword| input_lower.contains(keyword))
}

fn infer_weather_day(input_lower: &str) -> &'static str {
    if input_lower.contains("明天") || input_lower.contains("tomorrow") {
        "tomorrow"
    } else {
        "today"
    }
}

fn infer_weather_location(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut location = trimmed
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
        .trim()
        .to_string();

    if location.is_empty() {
        return None;
    }
    if location.ends_with('的') {
        location.pop();
    }
    Some(location.trim().to_string())
}

pub fn classify_query(user_input: &str) -> QueryKind {
    let input_lower = user_input.to_lowercase();
    if is_time_sensitive_current_query(&input_lower) {
        return QueryKind::TimeSensitiveCurrent;
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

    let allowed_tools: Vec<String> = match kind {
        QueryKind::TimeSensitiveCurrent => allowed_metadata
            .iter()
            .filter(|(_, metadata)| {
                matches!(metadata.scope, ToolScope::RemoteWeb | ToolScope::GitHub)
                    || metadata.intents.contains(&ToolIntent::Research)
                    || metadata.intents.contains(&ToolIntent::FetchWebPage)
                    || metadata.intents.contains(&ToolIntent::BrowseInteractive)
            })
            .map(|(name, _)| name.clone())
            .collect(),
        QueryKind::ExternalGitHubRepo => allowed_metadata
            .iter()
            .filter(|(_, metadata)| {
                matches!(metadata.scope, ToolScope::GitHub | ToolScope::RemoteWeb)
                    || metadata.intents.contains(&ToolIntent::Research)
                    || metadata.intents.contains(&ToolIntent::BrowseInteractive)
            })
            .map(|(name, _)| name.clone())
            .collect(),
        QueryKind::DirectExplanation => allowed_metadata
            .iter()
            .filter(|(_, metadata)| {
                !matches!(
                    metadata.scope,
                    ToolScope::LocalWorkspace | ToolScope::System
                ) && !metadata.intents.contains(&ToolIntent::ReadFile)
                    && !metadata.intents.contains(&ToolIntent::ReadCode)
                    && !metadata.intents.contains(&ToolIntent::ListDirectory)
                    && !metadata.intents.contains(&ToolIntent::RunCommand)
            })
            .map(|(name, _)| name.clone())
            .collect(),
        QueryKind::General => allowed_metadata
            .iter()
            .map(|(name, _)| name.clone())
            .collect(),
    };

    let system_hint = match kind {
        QueryKind::TimeSensitiveCurrent => {
            let today = Local::now().format("%Y-%m-%d").to_string();
            Some(format!("This is a time-sensitive current-information request. Today's date is {today}. Use fresh tools and prioritize results from {today}. Do not rely on stale memory or older summaries. If you answer with dates, mention exact dates explicitly."))
        }
        QueryKind::ExternalGitHubRepo => Some("This request is about an external GitHub repository. Prefer github_repo_inspect. Do not use local workspace tools like ls, cat, code_read, or shell unless the user explicitly asks about local files.".to_string()),
        QueryKind::DirectExplanation => Some("This is a direct explanation request. Answer directly from the conversation and available context. Do not call ls, cat, code_read, code_grep, or shell unless the user explicitly asks to inspect files or verify details.".to_string()),
        QueryKind::General => None,
    };

    ToolPolicyDecision {
        allowed_tools: if allowed_tools.is_empty() {
            allowed_metadata
                .iter()
                .map(|(name, _)| name.clone())
                .collect()
        } else {
            allowed_tools
        },
        system_hint,
    }
}

pub fn should_use_long_term_memory(user_input: &str) -> bool {
    !matches!(classify_query(user_input), QueryKind::TimeSensitiveCurrent)
}

pub fn rewrite_tool_call(tool: &str, args: &Value) -> ToolCallPolicyResult {
    let search_like_query = if tool == "deep_search" {
        args.get("topic").and_then(|value| value.as_str())
    } else if tool == "search" {
        args.get("url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.starts_with("http://") && !value.starts_with("https://"))
    } else {
        None
    };

    if let Some(query) = search_like_query {
        let lower = query.to_lowercase();
        if is_weather_query(&lower) {
            if let Some(location) = infer_weather_location(query) {
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
    }

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
    match classify_query(user_input) {
        QueryKind::TimeSensitiveCurrent => {
            if let Some(metadata) = tool_metadata {
                if matches!(
                    metadata.scope,
                    ToolScope::LocalWorkspace | ToolScope::System
                ) || metadata.intents.contains(&ToolIntent::ReadFile)
                    || metadata.intents.contains(&ToolIntent::ReadCode)
                    || metadata.intents.contains(&ToolIntent::ListDirectory)
                    || metadata.intents.contains(&ToolIntent::RunCommand)
                {
                    return Err(format!(
                        "This is a time-sensitive current-information request; use fresh web tools instead of {}.",
                        tool_name
                    ));
                }
            }
        }
        QueryKind::DirectExplanation => {
            if let Some(metadata) = tool_metadata {
                if matches!(
                    metadata.scope,
                    ToolScope::LocalWorkspace | ToolScope::System
                ) || metadata.intents.contains(&ToolIntent::ReadFile)
                    || metadata.intents.contains(&ToolIntent::ReadCode)
                    || metadata.intents.contains(&ToolIntent::ListDirectory)
                    || metadata.intents.contains(&ToolIntent::RunCommand)
                {
                    return Err(format!(
                        "This is a direct explanation request; answer directly instead of using {}.",
                        tool_name
                    ));
                }
            }
        }
        QueryKind::ExternalGitHubRepo => {
            if let Some(metadata) = tool_metadata {
                if matches!(
                    metadata.scope,
                    ToolScope::LocalWorkspace | ToolScope::System
                ) {
                    return Err(format!(
                        "This request targets an external GitHub repository; do not use local tool {}.",
                        tool_name
                    ));
                }
            }
        }
        QueryKind::General => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{ToolOutputShape, ToolRisk};

    fn local_file_tool() -> ToolMetadata {
        ToolMetadata::new(ToolScope::LocalWorkspace, vec![ToolIntent::ReadFile])
            .with_risk(ToolRisk::Low)
            .with_output_shape(ToolOutputShape::StructuredJson)
    }

    fn github_tool() -> ToolMetadata {
        ToolMetadata::new(ToolScope::GitHub, vec![ToolIntent::InspectRepository])
            .with_freshness(true)
            .with_output_shape(ToolOutputShape::StructuredJson)
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
        assert_eq!(
            classify_query("今天有什么新闻，推荐5条"),
            QueryKind::TimeSensitiveCurrent
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
    fn test_rewrite_search_to_github_repo_inspect() {
        let result = rewrite_tool_call(
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
    fn test_rewrite_deep_search_weather_to_weather_tool() {
        let result = rewrite_tool_call(
            "deep_search",
            &serde_json::json!({"topic": "吉隆坡明天天气"}),
        );
        assert_eq!(result.tool_name, "weather");
        assert_eq!(result.args["location"], "吉隆坡");
        assert_eq!(result.args["day"], "tomorrow");
    }
}
