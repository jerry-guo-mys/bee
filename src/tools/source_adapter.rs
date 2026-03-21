//! 内容源适配：统一识别 GitHub、社交帖文、搜索结果页、动态网页等内容源类型。

use regex::Regex;
use reqwest::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    GitHub,
    RepositoryFile,
    SocialContent,
    SearchResultsPage,
    DynamicWebPage,
    ArticlePage,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct SearchEngineTarget {
    pub engine: String,
    pub query: String,
}

fn extract_domain(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    parsed.host_str().map(|host| host.to_lowercase())
}

fn is_github_url(url: &str) -> bool {
    extract_domain(url)
        .map(|domain| domain == "github.com" || domain.ends_with(".github.com"))
        .unwrap_or(false)
}

fn is_social_url(url: &str) -> bool {
    extract_domain(url)
        .map(|domain| {
            matches!(
                domain.as_str(),
                "x.com"
                    | "www.x.com"
                    | "twitter.com"
                    | "www.twitter.com"
                    | "fixupx.com"
                    | "fxtwitter.com"
                    | "vxtwitter.com"
                    | "nitter.net"
                    | "reddit.com"
                    | "www.reddit.com"
            )
        })
        .unwrap_or(false)
}

pub fn search_engine_target(url: &str) -> Option<SearchEngineTarget> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_lowercase();
    let path = parsed.path().to_lowercase();
    let query_pairs: std::collections::HashMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let query = if host.contains("baidu.com") && path == "/s" {
        query_pairs.get("wd").cloned()
    } else if host.contains("google.") && path == "/search" {
        query_pairs.get("q").cloned()
    } else if host.contains("bing.com") && path == "/search" {
        query_pairs.get("q").cloned()
    } else {
        None
    }?;

    Some(SearchEngineTarget {
        engine: host,
        query,
    })
}

pub fn social_mirror_urls(url: &str) -> Vec<String> {
    let Ok(parsed) = Url::parse(url) else {
        return Vec::new();
    };
    let path = parsed.path().trim_start_matches('/');
    let query = parsed.query();

    ["fixupx.com", "fxtwitter.com", "vxtwitter.com", "nitter.net"]
        .iter()
        .filter_map(|host| {
            let mut mirror = Url::parse(&format!("https://{host}/")).ok()?;
            if !path.is_empty() {
                mirror.set_path(path);
            }
            mirror.set_query(query);
            Some(mirror.to_string())
        })
        .collect()
}

pub fn social_status_urls_from_text(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    if !(lower.contains("twitter") || lower.contains("x.com") || lower.contains("x/twitter")) {
        return Vec::new();
    }

    let Ok(id_re) = Regex::new(r"\b\d{12,}\b") else {
        return Vec::new();
    };
    let Some(status_id) = id_re.find(text).map(|m| m.as_str()) else {
        return Vec::new();
    };

    vec![
        format!("https://x.com/i/web/status/{status_id}"),
        format!("https://twitter.com/i/web/status/{status_id}"),
        format!("https://fixupx.com/i/web/status/{status_id}"),
        format!("https://fxtwitter.com/i/web/status/{status_id}"),
        format!("https://vxtwitter.com/i/web/status/{status_id}"),
        format!("https://nitter.net/i/web/status/{status_id}"),
    ]
}

pub fn classify_url_source(url: &str) -> SourceKind {
    if search_engine_target(url).is_some() {
        return SourceKind::SearchResultsPage;
    }

    if is_github_url(url) {
        let parsed = Url::parse(url).ok();
        let path = parsed.as_ref().map(|u| u.path()).unwrap_or_default();
        if path.contains("/blob/") || path.contains("/tree/") {
            return SourceKind::RepositoryFile;
        }
        return SourceKind::GitHub;
    }

    if is_social_url(url) {
        return SourceKind::SocialContent;
    }

    let Some(domain) = extract_domain(url) else {
        return SourceKind::Unknown;
    };

    if matches!(
        domain.as_str(),
        "medium.com"
            | "substack.com"
            | "www.substack.com"
            | "dev.to"
            | "hashnode.com"
            | "wikipedia.org"
    ) || domain.ends_with(".medium.com")
        || domain.ends_with(".substack.com")
        || domain.ends_with(".wikipedia.org")
    {
        return SourceKind::ArticlePage;
    }

    if matches!(
        domain.as_str(),
        "finance.yahoo.com" | "query1.finance.yahoo.com" | "open.er-api.com"
    ) {
        return SourceKind::DynamicWebPage;
    }

    SourceKind::ArticlePage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_search_results_page() {
        assert_eq!(
            classify_url_source("https://www.google.com/search?q=bee"),
            SourceKind::SearchResultsPage
        );
    }

    #[test]
    fn test_classify_social_content() {
        assert_eq!(
            classify_url_source("https://x.com/foo/status/123"),
            SourceKind::SocialContent
        );
    }

    #[test]
    fn test_social_status_urls_from_text() {
        let urls = social_status_urls_from_text("HiTw93 X/Twitter 2032091246588518683");
        assert!(urls.iter().any(|url| url.contains("nitter.net")));
    }
}
