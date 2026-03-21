use std::sync::Arc;

use async_trait::async_trait;
use html2text::from_read;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};

use crate::llm::LlmClient;
use crate::memory::Message;
use crate::tools::Tool;

pub struct DeepSearchTool {
    llm: Arc<dyn LlmClient>,
    client: Client,
    max_rounds: usize,
    max_results_per_round: usize,
    timeout_secs: u64,
    trusted_domains: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub query: String,
    pub content: String,
    pub source_url: String,
    pub relevance_score: f32,
    pub round: usize,
}

#[derive(Clone, Debug)]
pub struct DeepResearchResult {
    pub topic: String,
    pub search_results: Vec<SearchResult>,
    pub summary: String,
    pub key_findings: Vec<String>,
    pub follow_up_questions: Vec<String>,
}

impl DeepSearchTool {
    pub fn new(
        llm: Arc<dyn LlmClient>,
        max_rounds: usize,
        max_results_per_round: usize,
        timeout_secs: u64,
        trusted_domains: Vec<String>,
    ) -> Self {
        const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
        Self {
            llm,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent(USER_AGENT)
                .build()
                .unwrap_or_default(),
            max_rounds,
            max_results_per_round,
            timeout_secs,
            trusted_domains: trusted_domains
                .into_iter()
                .map(|d| d.to_lowercase())
                .collect(),
        }
    }

    fn extract_domain(url: &str) -> Option<String> {
        reqwest::Url::parse(url)
            .ok()?
            .host_str()
            .map(|host| host.to_lowercase())
    }

    fn is_trusted_domain(&self, url: &str) -> bool {
        let Some(domain) = Self::extract_domain(url) else {
            return false;
        };

        self.trusted_domains
            .iter()
            .any(|pattern| Self::domain_matches_pattern(&domain, pattern))
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
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn looks_like_html(body: &str) -> bool {
        let body = body.trim_start();
        body.starts_with("<!")
            || body.starts_with("<html")
            || body.starts_with("<HTML")
            || (body.contains("<body") || body.contains("<div") || body.contains("</p>"))
    }

    fn html_to_text(html: &str) -> String {
        match from_read(html.as_bytes(), 120) {
            Ok(text) if !text.trim().is_empty() => text,
            _ => Self::strip_html_tags(html),
        }
    }

    fn truncate_chars(s: &str, max_chars: usize) -> String {
        if s.chars().count() <= max_chars {
            s.to_string()
        } else {
            format!("{}...", s.chars().take(max_chars).collect::<String>())
        }
    }

    fn extract_first_json_object(s: &str) -> Option<&str> {
        let mut depth = 0;
        let mut start = None;
        let mut in_string = false;
        let mut escape_next = false;

        for (i, ch) in s.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start_idx) = start {
                            return Some(&s[start_idx..=i]);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn extract_candidate_urls(&self, html: &str) -> Vec<String> {
        let Ok(regex) = Regex::new(r#"https?://[^\s"'<>)]+"#) else {
            return Vec::new();
        };

        let mut urls = Vec::new();
        for m in regex.find_iter(html) {
            let candidate = m
                .as_str()
                .trim_end_matches(['"', '\'', ')', ']', '}', ',', '.'])
                .replace("&amp;", "&");

            if !self.is_trusted_domain(&candidate) {
                continue;
            }
            if urls.iter().any(|seen| seen == &candidate) {
                continue;
            }
            urls.push(candidate);
            if urls.len() >= self.max_results_per_round * 3 {
                break;
            }
        }
        urls
    }

    async fn search_query_urls(&self, query: &str) -> Result<Vec<String>, String> {
        let encoded = query.replace(' ', "+");
        let search_url = format!("https://www.bing.com/search?q={encoded}");
        let response = self
            .client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| format!("Search request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Search HTTP {}", response.status()));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read search response: {}", e))?;
        let urls = self.extract_candidate_urls(&body);
        if urls.is_empty() {
            Err("No trusted search results found".to_string())
        } else {
            Ok(urls)
        }
    }

    async fn fetch_page_text(&self, url: &str) -> Result<String, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Fetch failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("Fetch HTTP {}", response.status()));
        }

        let mut body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read page body: {}", e))?;
        if body.starts_with('\u{FEFF}') {
            body = body[1..].to_string();
        }

        let text = if Self::looks_like_html(&body) {
            Self::html_to_text(&body)
        } else {
            body
        };
        Ok(Self::truncate_chars(text.trim(), 4000))
    }

    async fn decompose_query(&self, query: &str) -> Result<Vec<String>, String> {
        let prompt = format!(
            r#"You are a research assistant. Break down the following complex research question into 3-5 specific, searchable sub-questions.
Each sub-question should be:
- Specific and focused
- Suitable for web search
- Cover different aspects of the main topic

Research question: {}

Output format (JSON array of strings):
["sub-question 1", "sub-question 2", "sub-question 3"]

Sub-questions:"#,
            query
        );

        let messages = vec![Message::user(&prompt)];
        let response = self
            .llm
            .complete(&messages)
            .await
            .map_err(|e| format!("LLM error: {}", e))?;

        let queries: Vec<String> =
            serde_json::from_str(&response).unwrap_or_else(|_| vec![query.to_string()]);

        Ok(queries.into_iter().take(5).collect())
    }

    async fn search_round(
        &self,
        queries: &[String],
        round: usize,
    ) -> Result<Vec<SearchResult>, String> {
        let mut results = Vec::new();

        for query in queries {
            let urls = match self.search_query_urls(query).await {
                Ok(urls) => urls,
                Err(err) => {
                    tracing::warn!(query = %query, error = %err, "deep_search query failed");
                    continue;
                }
            };

            for url in urls {
                let content = match self.fetch_page_text(&url).await {
                    Ok(content) if !content.trim().is_empty() => content,
                    Ok(_) => continue,
                    Err(err) => {
                        tracing::warn!(url = %url, error = %err, "deep_search fetch failed");
                        continue;
                    }
                };

                let relevance_score = if content.to_lowercase().contains(&query.to_lowercase()) {
                    0.9
                } else {
                    0.7
                };

                results.push(SearchResult {
                    query: query.clone(),
                    content,
                    source_url: url,
                    relevance_score,
                    round,
                });

                if results.len() >= self.max_results_per_round {
                    break;
                }
            }

            if results.len() >= self.max_results_per_round {
                break;
            }
        }

        results.truncate(self.max_results_per_round);
        Ok(results)
    }

    async fn generate_follow_up_queries(
        &self,
        original_query: &str,
        previous_results: &[SearchResult],
    ) -> Result<Vec<String>, String> {
        let results_summary: String = previous_results
            .iter()
            .take(3)
            .map(|r| {
                format!(
                    "- {}: {}",
                    r.query,
                    r.content.chars().take(200).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"Based on the initial research, generate 2-3 follow-up search queries to deepen understanding.
Original query: {}

Previous findings:
{}

Output format (JSON array):
["follow-up query 1", "follow-up query 2"]

Follow-up queries:"#,
            original_query, results_summary
        );

        let messages = vec![Message::user(&prompt)];
        let response = self
            .llm
            .complete(&messages)
            .await
            .map_err(|e| format!("LLM error: {}", e))?;

        let queries: Vec<String> = serde_json::from_str(&response).unwrap_or_else(|_| vec![]);

        Ok(queries.into_iter().take(3).collect())
    }

    async fn synthesize_results(
        &self,
        topic: &str,
        results: &[SearchResult],
    ) -> Result<(String, Vec<String>, Vec<String>), String> {
        let findings: String = results
            .iter()
            .map(|r| {
                format!(
                    "Source: {}\n{}",
                    r.source_url,
                    r.content.chars().take(500).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let prompt = format!(
            r#"Synthesize the following research findings into a comprehensive summary.

Topic: {}

Research findings:
{}

Output format (JSON):
{{
    "summary": "200-300 word comprehensive summary",
    "key_findings": ["finding 1", "finding 2", "finding 3"],
    "follow_up_questions": ["question 1", "question 2"]
}}

Synthesis:"#,
            topic, findings
        );

        let messages = vec![Message::user(&prompt)];
        let response = self
            .llm
            .complete(&messages)
            .await
            .map_err(|e| format!("LLM error: {}", e))?;

        let synthesis_text = Self::extract_first_json_object(&response).unwrap_or(&response);
        let synthesis: Value = match serde_json::from_str(synthesis_text) {
            Ok(value) => value,
            Err(_) => {
                return Ok((
                    Self::truncate_chars(response.trim(), 1200),
                    Vec::new(),
                    Vec::new(),
                ));
            }
        };

        let summary = synthesis["summary"]
            .as_str()
            .unwrap_or("No summary available")
            .to_string();
        let key_findings = synthesis["key_findings"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let follow_up_questions = synthesis["follow_up_questions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok((summary, key_findings, follow_up_questions))
    }
}

#[async_trait]
impl Tool for DeepSearchTool {
    fn name(&self) -> &str {
        "deep_search"
    }

    fn description(&self) -> &str {
        "Conduct deep research on a complex topic through multiple rounds of autonomous search. Automatically decomposes query, performs iterative searches, and synthesizes findings. Args: {\"topic\": \"research question\", \"max_rounds\": 3 (optional)}"
    }

    fn timeout_secs(&self) -> Option<u64> {
        Some(self.timeout_secs)
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let topic = args
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if topic.is_empty() {
            return Err("Missing topic".to_string());
        }

        let max_rounds = args.get("max_rounds").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        tracing::info!(topic = %topic, max_rounds = max_rounds, "deep_search started");

        let max_rounds = max_rounds.min(self.max_rounds);

        let initial_queries = self.decompose_query(topic).await?;
        tracing::info!(queries = ?initial_queries, "decomposed into queries");

        let mut all_results: Vec<SearchResult> = Vec::new();
        let mut current_queries = initial_queries;

        for round in 1..=max_rounds {
            tracing::info!(round, "starting search round");

            let round_results = self.search_round(&current_queries, round).await?;
            all_results.extend(round_results);

            if round < max_rounds {
                current_queries = self.generate_follow_up_queries(topic, &all_results).await?;
                if current_queries.is_empty() {
                    break;
                }
            }
        }

        let (summary, key_findings, follow_up_questions) =
            self.synthesize_results(topic, &all_results).await?;

        let result = DeepResearchResult {
            topic: topic.to_string(),
            search_results: all_results,
            summary,
            key_findings,
            follow_up_questions,
        };

        let output = json!({
            "topic": result.topic,
            "summary": result.summary,
            "key_findings": result.key_findings,
            "total_sources": result.search_results.len(),
            "sources": result.search_results.iter().map(|r| {
                json!({
                    "query": r.query,
                    "url": r.source_url,
                    "relevance_score": r.relevance_score,
                    "round": r.round,
                })
            }).collect::<Vec<_>>(),
            "follow_up_questions": result.follow_up_questions,
        });

        Ok(output.to_string())
    }
}
