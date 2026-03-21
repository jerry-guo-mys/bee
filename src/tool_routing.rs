//! 工具路由收敛：根据用户问题缩小候选工具集合，减少语义重叠与误选。

use serde_json::Value;

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

pub fn is_external_github_repo_query(user_input: &str) -> bool {
    let input_lower = user_input.to_lowercase();
    let Some(url) = extract_url(user_input) else {
        return false;
    };
    if !is_github_repo_url(&url) || contains_local_workspace_signal(&input_lower) {
        return false;
    }
    is_github_architecture_query(&input_lower) || is_github_locator_query(&input_lower)
}

pub fn refine_allowed_tools_for_input(user_input: &str, allowed_tools: &[String]) -> Vec<String> {
    let input_lower = user_input.to_lowercase();

    if !is_external_github_repo_query(user_input) {
        if is_direct_explanation_query(&input_lower) {
            return allowed_tools
                .iter()
                .filter(|tool| {
                    !matches!(
                        tool.as_str(),
                        "ls" | "cat" | "code_read" | "code_grep" | "shell"
                    )
                })
                .cloned()
                .collect();
        }
        return allowed_tools.to_vec();
    }

    let preferred = ["github_repo_inspect", "search", "browser", "deep_search"];
    let mut refined = Vec::new();

    for tool in preferred {
        if allowed_tools.iter().any(|name| name == tool) {
            refined.push(tool.to_string());
        }
    }

    if refined.is_empty() {
        allowed_tools.to_vec()
    } else {
        refined
    }
}

pub fn system_hint_for_input(user_input: &str) -> Option<String> {
    let input_lower = user_input.to_lowercase();

    if is_external_github_repo_query(user_input) {
        return Some("This request is about an external GitHub repository. Prefer github_repo_inspect. Do not use local workspace tools like ls, cat, code_read, or shell unless the user explicitly asks about local files.".to_string());
    }

    if is_direct_explanation_query(&input_lower) {
        return Some("This is a direct explanation request. Answer directly from the conversation and available context. Do not call ls, cat, code_read, code_grep, or shell unless the user explicitly asks to inspect files or verify details.".to_string());
    }

    None
}

pub fn rewrite_tool_call(tool: &str, args: &Value) -> Option<(String, Value)> {
    if tool != "search" {
        return None;
    }

    let url = args.get("url").and_then(|value| value.as_str())?;
    if !is_github_repo_url(url) {
        return None;
    }

    Some((
        "github_repo_inspect".to_string(),
        serde_json::json!({ "url": url }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_external_github_repo_query() {
        assert!(is_external_github_repo_query(
            "分析 https://github.com/paperclipai/paperclip 的技术架构"
        ));
        assert!(!is_external_github_repo_query(
            "读取当前 workspace 里的 package.json"
        ));
    }

    #[test]
    fn test_refine_allowed_tools_for_github_query() {
        let allowed = vec![
            "cat".to_string(),
            "code_read".to_string(),
            "search".to_string(),
            "github_repo_inspect".to_string(),
            "ls".to_string(),
        ];

        let refined = refine_allowed_tools_for_input(
            "看一下 https://github.com/paperclipai/paperclip 的 package.json 和技术架构",
            &allowed,
        );

        assert_eq!(
            refined,
            vec!["github_repo_inspect".to_string(), "search".to_string()]
        );
    }

    #[test]
    fn test_refine_allowed_tools_for_direct_explanation_query() {
        let allowed = vec![
            "ls".to_string(),
            "cat".to_string(),
            "code_read".to_string(),
            "search".to_string(),
            "github_repo_inspect".to_string(),
        ];

        let refined =
            refine_allowed_tools_for_input("Paperclip 是什么产品？核心功能是什么？", &allowed);

        assert_eq!(
            refined,
            vec!["search".to_string(), "github_repo_inspect".to_string()]
        );
    }

    #[test]
    fn test_system_hint_for_direct_explanation_query() {
        let hint = system_hint_for_input("这个服务是做什么的？").unwrap_or_default();
        assert!(hint.contains("direct explanation request"));
    }

    #[test]
    fn test_rewrite_search_github_repo_to_github_repo_inspect() {
        let args = serde_json::json!({
            "url": "https://github.com/paperclipai/paperclip"
        });
        let rewritten = rewrite_tool_call("search", &args).unwrap();
        assert_eq!(rewritten.0, "github_repo_inspect");
        assert_eq!(
            rewritten.1["url"],
            "https://github.com/paperclipai/paperclip"
        );
    }
}
