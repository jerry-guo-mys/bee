//! Search/Web 工具：域名白名单、超时、结果大小限制
//!
//! 仅允许配置中的域名（如 wikipedia、docs.rs）；GET 请求带超时与 User-Agent；
//! 响应超过 max_result_chars 时截断并追加 ...[truncated]。
//! 对 HTML 响应使用 html2text 提取可读文本，去除标签与脚本。

use std::collections::HashSet;

use async_trait::async_trait;
use html2text::from_read;
use reqwest::Client;
use serde_json::json;
use serde_json::Value;

use crate::tools::output;
use crate::tools::source_adapter::{classify_url_source, social_mirror_urls, SearchEngineTarget};
use crate::tools::{
    search_engine_target, SourceKind, Tool, ToolCapabilityGroup, ToolCapabilitySubgroup,
    ToolCostClass, ToolCriticMode, ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape,
    ToolRisk, ToolScope, ToolUseCase,
};

/// Search 工具：抓取 URL 内容，仅允许白名单域名；超时与最大字符数由配置决定
pub struct SearchTool {
    client: Client,
    allowed_domains: HashSet<String>,
    max_result_chars: usize,
}

fn domain_matches_pattern(domain: &str, pattern: &str) -> bool {
    let domain = domain.to_lowercase();
    let pattern = pattern.trim().to_lowercase();

    if pattern.is_empty() {
        return false;
    }

    if let Some(suffix) = pattern.strip_prefix("*.") {
        return domain == suffix || domain.ends_with(&format!(".{suffix}"));
    }

    domain == pattern || domain.ends_with(&format!(".{pattern}"))
}

/// 简易去除 HTML 标签（html2text 失败时的回退）
fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut prev_whitespace = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => {
                let is_whitespace = c.is_whitespace();
                if is_whitespace && prev_whitespace {
                    continue;
                }
                prev_whitespace = is_whitespace;
                out.push(if is_whitespace { ' ' } else { c });
            }
            _ => {}
        }
    }
    out.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// 判断内容是否像 HTML（需提取可读文本）
fn looks_like_html(s: &str) -> bool {
    let s = s.trim_start();
    s.starts_with("<!")
        || s.starts_with("<html")
        || s.starts_with("<HTML")
        || (s.len() > 20
            && s.contains('<')
            && (s.contains("</")
                || s.contains("<meta")
                || s.contains("<head")
                || s.contains("<title")))
}

/// 从 URL 中提取 host（不含端口后的路径）
fn extract_domain(url: &str) -> Option<String> {
    let url = url.trim();
    let url = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = url.split('/').next()?;
    let host = host.split(':').next()?;
    Some(host.to_lowercase())
}

fn is_blocked_host(domain: &str) -> bool {
    let lower = domain.to_lowercase();
    if matches!(lower.as_str(), "localhost" | "0.0.0.0") || lower.ends_with(".local") {
        return true;
    }
    if lower.starts_with("127.") || lower.starts_with("10.") || lower.starts_with("192.168.") {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("172.") {
        if let Some(octet) = rest.split('.').next() {
            if let Ok(value) = octet.parse::<u8>() {
                if (16..=31).contains(&value) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_x_js_placeholder(text: &str) -> bool {
    let lower = text.to_lowercase();
    (lower.contains("javascript is disabled") || lower.contains("enable javascript"))
        && (lower.contains("x.com")
            || lower.contains("twitter")
            || lower.contains("x platform")
            || lower.contains("switch to a supported browser"))
}

fn source_kind_label(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::GitHub => "github",
        SourceKind::RepositoryFile => "repository_file",
        SourceKind::SocialContent => "social_content",
        SourceKind::SearchResultsPage => "search_results_page",
        SourceKind::DynamicWebPage => "dynamic_web_page",
        SourceKind::ArticlePage => "article_page",
        SourceKind::Unknown => "unknown",
    }
}

fn is_recoverable_fetch_error(error: &str) -> bool {
    error.starts_with("HTTP 404") || error.starts_with("HTTP 410") || error.starts_with("HTTP 451")
}

fn suggested_next_steps(kind: SourceKind, url: &str) -> Vec<String> {
    match kind {
        SourceKind::ArticlePage | SourceKind::DynamicWebPage => vec![
            "Try the site homepage or category page instead of the dead deep link.".to_string(),
            "Search by article title, topic, or site name to find the updated page.".to_string(),
            format!(
                "Verify whether the article at {} has moved or been removed.",
                url
            ),
        ],
        SourceKind::SocialContent => vec![
            "Try a readable mirror or another source that quotes the same post.".to_string(),
            "Search by author handle plus post ID or quoted text.".to_string(),
        ],
        SourceKind::SearchResultsPage => vec![
            "Refine the search query and fetch a landing page instead of the search results page."
                .to_string(),
        ],
        _ => vec!["Try a broader search or a higher-level page for the same source.".to_string()],
    }
}

impl SearchTool {
    pub fn new(allowed_domains: Vec<String>, timeout_secs: u64, max_result_chars: usize) -> Self {
        let allowed_domains = allowed_domains
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect();
        // 使用现代浏览器 UA 与常用请求头，避免被站点识别为低版本或爬虫
        const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .user_agent(USER_AGENT)
            .default_headers({
                use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE};
                let mut h = reqwest::header::HeaderMap::new();
                h.insert(
                    ACCEPT,
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
                        .parse()
                        .unwrap(),
                );
                h.insert(ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap());
                h
            })
            .build()
            .unwrap_or_default();
        Self {
            client,
            allowed_domains,
            max_result_chars,
        }
    }

    fn is_allowed(&self, url: &str) -> Result<(), String> {
        let domain = extract_domain(url).ok_or_else(|| "Invalid or missing URL".to_string())?;
        if is_blocked_host(&domain) {
            return Err(format!("Blocked internal domain: {}", domain));
        }
        if self.allowed_domains.is_empty() || self.allowed_domains.contains("*") {
            return Ok(());
        }
        if self
            .allowed_domains
            .iter()
            .any(|pattern| domain_matches_pattern(&domain, pattern))
        {
            return Ok(());
        }
        Err(format!("Domain not in allowlist: {}", domain))
    }

    /// 将 HTML 转为可读文本（去除 script/style 等）
    fn html_to_text(&self, html: &str) -> String {
        match from_read(html.as_bytes(), 120) {
            Ok(text) if !text.trim().is_empty() => text,
            _ => strip_html_tags(html),
        }
    }

    async fn fetch_page_text(&self, url: &str) -> Result<String, String> {
        self.is_allowed(url)?;
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let mut body = resp.text().await.map_err(|e| format!("Read body: {}", e))?;
        if body.starts_with('\u{FEFF}') {
            body = body[1..].to_string();
        }
        let text = if looks_like_html(&body) {
            self.html_to_text(&body)
        } else {
            body
        };
        Ok(text)
    }

    async fn fetch_x_with_fallback(&self, url: &str) -> Result<String, String> {
        let primary = self.fetch_page_text(url).await?;
        if !is_x_js_placeholder(&primary) {
            return Ok(primary);
        }

        for mirror_url in social_mirror_urls(url) {
            if let Ok(text) = self.fetch_page_text(&mirror_url).await {
                if !text.trim().is_empty() && !is_x_js_placeholder(&text) {
                    return Ok(format!("[Fetched via mirror: {}]\n\n{}", mirror_url, text));
                }
            }
        }

        Ok(primary)
    }

    fn extract_candidate_urls(&self, html: &str, engine_host: &str) -> Vec<String> {
        let Ok(regex) = regex::Regex::new(r#"https?://[^\s"'<>)]+"#) else {
            return Vec::new();
        };

        let mut urls = Vec::new();
        for matched in regex.find_iter(html) {
            let candidate = matched
                .as_str()
                .trim_end_matches(['"', '\'', ')', ']', '}', ',', '.'])
                .replace("&amp;", "&");
            let Some(domain) = extract_domain(&candidate) else {
                continue;
            };
            if domain.contains(engine_host) || is_blocked_host(&domain) {
                continue;
            }
            if urls.iter().any(|seen| seen == &candidate) {
                continue;
            }
            urls.push(candidate);
            if urls.len() >= 5 {
                break;
            }
        }
        urls
    }

    async fn fetch_search_engine_results(
        &self,
        url: &str,
        target: &SearchEngineTarget,
    ) -> Result<String, String> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let html = resp.text().await.map_err(|e| format!("Read body: {}", e))?;
        let candidate_urls = self.extract_candidate_urls(&html, &target.engine);
        let mut candidates = Vec::new();

        for candidate_url in candidate_urls {
            if let Ok(text) = self.fetch_page_text(&candidate_url).await {
                let snippet = if text.chars().count() > 400 {
                    format!("{}...", text.chars().take(400).collect::<String>())
                } else {
                    text
                };
                candidates.push(serde_json::json!({
                    "url": candidate_url,
                    "snippet": snippet,
                }));
            }
        }

        output::structured(
            self.name(),
            format!("Fetched search results for {}", target.query),
            false,
            serde_json::json!({
                "search_engine": target.engine,
                "query": target.query,
                "candidates": candidates,
            }),
        )
    }

    async fn fetch(&self, url: &str) -> Result<String, String> {
        self.is_allowed(url)?;
        let body = match classify_url_source(url) {
            SourceKind::GitHub | SourceKind::RepositoryFile => {
                return Err(
                    "GitHub repository URLs should use github_repo_inspect, not search".to_string(),
                );
            }
            SourceKind::SearchResultsPage => {
                let target = search_engine_target(url)
                    .ok_or_else(|| "Invalid search results page".to_string())?;
                return self.fetch_search_engine_results(url, &target).await;
            }
            SourceKind::SocialContent => self.fetch_x_with_fallback(url).await?,
            SourceKind::DynamicWebPage | SourceKind::ArticlePage | SourceKind::Unknown => {
                self.fetch_page_text(url).await?
            }
        };

        let len = body.chars().count();
        if len > self.max_result_chars {
            Ok(body.chars().take(self.max_result_chars).collect::<String>() + "\n...[truncated]")
        } else {
            Ok(body)
        }
    }

    fn build_recoverable_error_output(&self, url: &str, error: &str) -> Result<String, String> {
        let kind = classify_url_source(url);
        output::structured(
            self.name(),
            format!("Page fetch failed for {}: {}", url, error),
            false,
            json!({
                "url": url,
                "error": error,
                "recoverable": true,
                "source_kind": source_kind_label(kind),
                "suggested_next_steps": suggested_next_steps(kind, url),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::source_adapter::{classify_url_source, social_mirror_urls, SourceKind};

    #[test]
    fn test_detect_x_or_twitter_url() {
        assert_eq!(
            classify_url_source("https://x.com/foo/status/123"),
            SourceKind::SocialContent
        );
        assert_eq!(
            classify_url_source("https://twitter.com/foo/status/123"),
            SourceKind::SocialContent
        );
        assert_eq!(
            classify_url_source("https://example.com/foo"),
            SourceKind::ArticlePage
        );
    }

    #[test]
    fn test_detect_x_js_placeholder() {
        assert!(is_x_js_placeholder(
            "JavaScript is disabled in this browser. Please enable JavaScript or switch to a supported browser to continue using x.com"
        ));
        assert!(!is_x_js_placeholder("regular readable page content"));
    }

    #[test]
    fn test_build_x_mirror_urls() {
        let urls = social_mirror_urls("https://x.com/HiTw93/status/2032091246588518683");
        assert_eq!(urls.len(), 4);
        assert!(urls[0].contains("fixupx.com/HiTw93/status/2032091246588518683"));
        assert!(urls
            .iter()
            .any(|u| u.contains("nitter.net/HiTw93/status/2032091246588518683")));
    }

    #[test]
    fn test_detect_recoverable_fetch_error() {
        assert!(is_recoverable_fetch_error("HTTP 404 Not Found"));
        assert!(is_recoverable_fetch_error("HTTP 410 Gone"));
        assert!(!is_recoverable_fetch_error("HTTP 403 Forbidden"));
        assert!(!is_recoverable_fetch_error("Request failed: timeout"));
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Fetch URL content and extract readable text from general web pages on the allowlist. Use github_repo_inspect for GitHub repository, blob, or tree URLs. Args: {\"url\": \"https://...\"}."
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolScope::RemoteWeb, vec![ToolIntent::FetchWebPage])
            .with_risk(ToolRisk::Low)
            .with_output_shape(ToolOutputShape::StructuredJson)
            .with_freshness(ToolFreshness::BestEffort)
            .with_preferred_use_cases(vec![
                ToolUseCase::DirectExplanation,
                ToolUseCase::TimeSensitiveCurrent,
            ])
            .with_capability(
                ToolCapabilityGroup::WebResearch,
                ToolCapabilitySubgroup::ArticlePage,
            )
            .with_costs(
                ToolCostClass::Low,
                ToolCostClass::Low,
                ToolCostClass::Low,
                ToolCostClass::Low,
            )
            .with_preferred_rank(3)
            .with_critic_mode(ToolCriticMode::Conservative)
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if url.is_empty() {
            return Err("Missing url".to_string());
        }
        tracing::info!(url = %url, "search tool fetch");
        if search_engine_target(url).is_some() {
            return self.fetch(url).await;
        }
        let content = match self.fetch(url).await {
            Ok(content) => content,
            Err(error) if is_recoverable_fetch_error(&error) => {
                return self.build_recoverable_error_output(url, &error);
            }
            Err(error) => return Err(error),
        };
        output::structured(
            self.name(),
            format!("Fetched web content from {}", url),
            false,
            serde_json::json!({
                "url": url,
                "content": content,
            }),
        )
    }
}
