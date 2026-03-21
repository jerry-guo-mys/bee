use async_trait::async_trait;
use reqwest::{Client, Url};
use serde_json::Value;

use crate::tools::output;
use crate::tools::{
    Tool, ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode,
    ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};

pub struct NewsTool {
    client: Client,
    timeout_secs: u64,
}

impl NewsTool {
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

    fn build_url(&self, query: Option<&str>) -> Result<Url, String> {
        let mut url = if let Some(query) = query.filter(|q| !q.trim().is_empty()) {
            let mut url = Url::parse("https://news.google.com/rss/search")
                .map_err(|e| format!("Invalid news URL: {}", e))?;
            url.query_pairs_mut()
                .append_pair("q", query)
                .append_pair("hl", "zh-CN")
                .append_pair("gl", "CN")
                .append_pair("ceid", "CN:zh-Hans");
            url
        } else {
            Url::parse("https://news.google.com/rss?hl=zh-CN&gl=CN&ceid=CN:zh-Hans")
                .map_err(|e| format!("Invalid news URL: {}", e))?
        };
        url.query_pairs_mut().append_pair("oc", "5");
        Ok(url)
    }

    fn strip_cdata(text: &str) -> String {
        text.replace("<![CDATA[", "")
            .replace("]]>", "")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .trim()
            .to_string()
    }

    fn extract_tag(block: &str, tag: &str) -> Option<String> {
        let start = format!("<{tag}>");
        let end = format!("</{tag}>");
        let start_idx = block.find(&start)? + start.len();
        let end_idx = block[start_idx..].find(&end)? + start_idx;
        Some(Self::strip_cdata(&block[start_idx..end_idx]))
    }

    fn extract_source(block: &str) -> Option<String> {
        let source_start = block.find("<source")?;
        let gt = block[source_start..].find('>')? + source_start + 1;
        let end = block[gt..].find("</source>")? + gt;
        Some(Self::strip_cdata(&block[gt..end]))
    }

    fn parse_items(xml: &str, limit: usize) -> Vec<Value> {
        let mut items = Vec::new();
        let mut remaining = xml;

        while let Some(start) = remaining.find("<item>") {
            let after_start = &remaining[start + "<item>".len()..];
            let Some(end) = after_start.find("</item>") else {
                break;
            };
            let block = &after_start[..end];
            items.push(serde_json::json!({
                "title": Self::extract_tag(block, "title"),
                "link": Self::extract_tag(block, "link"),
                "published_at": Self::extract_tag(block, "pubDate"),
                "source": Self::extract_source(block),
            }));
            if items.len() >= limit {
                break;
            }
            remaining = &after_start[end + "</item>".len()..];
        }

        items
    }
}

#[async_trait]
impl Tool for NewsTool {
    fn name(&self) -> &str {
        "news"
    }

    fn description(&self) -> &str {
        "Get current news headlines. Args: {\"query\": \"optional topic\", \"limit\": 5}"
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::RemoteWeb,
            vec![ToolIntent::FetchWebPage, ToolIntent::Research],
        )
        .with_risk(ToolRisk::Low)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_freshness(ToolFreshness::Live)
        .with_preferred_use_cases(vec![ToolUseCase::News, ToolUseCase::TimeSensitiveCurrent])
        .with_capability(
            ToolCapabilityGroup::RealtimeData,
            ToolCapabilitySubgroup::News,
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
                "query": {
                    "type": "string",
                    "description": "Optional topic or keyword"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum headlines to return",
                    "default": 5
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let query = args.get("query").and_then(|v| v.as_str()).map(str::trim);
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(5)
            .clamp(1, 10) as usize;
        let url = self.build_url(query)?;
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("News request failed: {}", e))?;
        let response = response
            .error_for_status()
            .map_err(|e| format!("News request failed: {}", e))?;
        let xml = response
            .text()
            .await
            .map_err(|e| format!("News response parse failed: {}", e))?;
        let items = Self::parse_items(&xml, limit);

        output::structured(
            self.name(),
            format!("Fetched {} news headlines", items.len()),
            !items.is_empty(),
            serde_json::json!({
                "query": query,
                "items": items,
                "source": "Google News RSS"
            }),
        )
    }
}
