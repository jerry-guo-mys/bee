use std::collections::BTreeSet;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

use crate::tools::output;
use crate::tools::{
    Tool, ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode,
    ToolFreshness, ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};

pub struct GitHubRepoInspectTool {
    client: Client,
    max_result_chars: usize,
}

enum GitHubTarget {
    Repo {
        owner: String,
        repo: String,
    },
    Blob {
        owner: String,
        repo: String,
        branch: String,
        path: String,
    },
    Tree {
        owner: String,
        repo: String,
        branch: String,
        path: String,
    },
}

impl GitHubRepoInspectTool {
    pub fn new(max_result_chars: usize) -> Self {
        const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .user_agent(USER_AGENT)
                .build()
                .unwrap_or_default(),
            max_result_chars,
        }
    }

    fn parse_target(input: &str) -> Option<GitHubTarget> {
        let url = input
            .trim()
            .strip_prefix("https://github.com/")
            .or_else(|| input.trim().strip_prefix("http://github.com/"))?;
        let mut parts = url.split('/').filter(|part| !part.is_empty());
        let owner = parts.next()?.trim();
        let repo = parts.next()?.trim();
        if owner.is_empty() || repo.is_empty() {
            return None;
        }
        let action = parts.next().unwrap_or_default();
        let branch = parts.next().unwrap_or_default();
        let rest = parts.collect::<Vec<_>>().join("/");

        match action {
            "blob" if !branch.is_empty() && !rest.is_empty() => Some(GitHubTarget::Blob {
                owner: owner.to_string(),
                repo: repo.to_string(),
                branch: branch.to_string(),
                path: rest,
            }),
            "tree" if !branch.is_empty() => Some(GitHubTarget::Tree {
                owner: owner.to_string(),
                repo: repo.to_string(),
                branch: branch.to_string(),
                path: rest,
            }),
            _ => Some(GitHubTarget::Repo {
                owner: owner.to_string(),
                repo: repo.to_string(),
            }),
        }
    }

    async fn fetch_json(&self, url: &str) -> Result<Value, String> {
        let resp = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.json::<Value>()
            .await
            .map_err(|e| format!("JSON parse failed: {}", e))
    }

    async fn fetch_text(&self, url: &str) -> Result<String, String> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.text().await.map_err(|e| format!("Read body: {}", e))
    }

    fn truncate_chars(&self, s: &str, limit: usize) -> String {
        if s.chars().count() <= limit {
            s.to_string()
        } else {
            format!("{}...", s.chars().take(limit).collect::<String>())
        }
    }

    fn detect_stack(paths: &[String]) -> Vec<String> {
        let mut stack = BTreeSet::new();
        for path in paths {
            match path.as_str() {
                p if p.ends_with("package.json") => {
                    stack.insert("Node.js / JavaScript ecosystem".to_string());
                }
                p if p.ends_with("pnpm-workspace.yaml") => {
                    stack.insert("pnpm workspace".to_string());
                }
                p if p.ends_with("turbo.json") => {
                    stack.insert("Turborepo".to_string());
                }
                p if p.ends_with("Cargo.toml") => {
                    stack.insert("Rust / Cargo".to_string());
                }
                p if p.ends_with("go.mod") => {
                    stack.insert("Go modules".to_string());
                }
                p if p.ends_with("pyproject.toml") || p.ends_with("requirements.txt") => {
                    stack.insert("Python".to_string());
                }
                p if p.ends_with("docker-compose.yml")
                    || p.ends_with("docker-compose.yaml")
                    || p.ends_with("Dockerfile") =>
                {
                    stack.insert("Docker".to_string());
                }
                _ => {}
            }
        }
        stack.into_iter().collect()
    }

    fn build_repo_summary(
        description: &str,
        detected_stack: &[String],
        top_level_dirs: &[String],
        selected_paths: &[String],
    ) -> String {
        let mut parts = Vec::new();
        if !description.trim().is_empty() {
            parts.push(format!("Project purpose: {}", description.trim()));
        }
        if !detected_stack.is_empty() {
            parts.push(format!("Likely stack: {}", detected_stack.join(", ")));
        }
        if !top_level_dirs.is_empty() {
            parts.push(format!(
                "Top-level areas: {}",
                top_level_dirs
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !selected_paths.is_empty() {
            parts.push(format!(
                "Key architecture files: {}",
                selected_paths
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        parts.join(". ")
    }

    async fn inspect_repo(&self, owner: &str, repo: &str) -> Result<Value, String> {
        let repo_meta = self
            .fetch_json(&format!("https://api.github.com/repos/{owner}/{repo}"))
            .await?;
        let branch = repo_meta["default_branch"].as_str().unwrap_or("main");
        let tree = self
            .fetch_json(&format!(
                "https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1"
            ))
            .await?;

        let paths: Vec<String> = tree["tree"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["path"].as_str().map(|s| s.to_string()))
            .collect();

        let mut top_level_dirs = BTreeSet::new();
        for path in &paths {
            if let Some((head, _)) = path.split_once('/') {
                top_level_dirs.insert(head.to_string());
            }
        }
        let top_level_dirs_vec = top_level_dirs.into_iter().collect::<Vec<_>>();

        let key_file_patterns = [
            "README.md",
            "ARCHITECTURE.md",
            "docs/architecture.md",
            "docs/ARCHITECTURE.md",
            "package.json",
            "pnpm-workspace.yaml",
            "turbo.json",
            "Cargo.toml",
            "go.mod",
            "pyproject.toml",
            "requirements.txt",
            "docker-compose.yml",
            "docker-compose.yaml",
            "Dockerfile",
            "apps/web/package.json",
            "apps/api/package.json",
            "server/package.json",
            "backend/package.json",
            "frontend/package.json",
        ];

        let mut selected_paths = Vec::new();
        for pattern in key_file_patterns {
            for path in &paths {
                if path == pattern || path.ends_with(&format!("/{pattern}")) {
                    if !selected_paths.iter().any(|p| p == path) {
                        selected_paths.push(path.clone());
                    }
                }
            }
        }
        selected_paths.truncate(10);

        let mut snippets = Vec::new();
        for path in &selected_paths {
            let raw_url =
                format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}");
            if let Ok(body) = self.fetch_text(&raw_url).await {
                let trimmed = body.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("404: Not Found") {
                    snippets.push(json!({
                        "path": path,
                        "content": self.truncate_chars(trimmed, 1200),
                    }));
                }
            }
        }

        let detected_stack = Self::detect_stack(&paths);
        let repo_summary = Self::build_repo_summary(
            repo_meta["description"].as_str().unwrap_or(""),
            &detected_stack,
            &top_level_dirs_vec,
            &selected_paths,
        );

        Ok(json!({
            "target_type": "repo",
            "repository": format!("{owner}/{repo}"),
            "description": repo_meta["description"].as_str().unwrap_or(""),
            "default_branch": branch,
            "language": repo_meta["language"].as_str().unwrap_or(""),
            "topics": repo_meta["topics"].as_array().cloned().unwrap_or_default(),
            "repo_summary": repo_summary,
            "detected_stack": detected_stack,
            "top_level_directories": top_level_dirs_vec,
            "key_files_found": selected_paths,
            "file_snippets": snippets,
        }))
    }

    async fn inspect_blob(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<Value, String> {
        let raw_url = format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{path}");
        let content = self.fetch_text(&raw_url).await?;
        Ok(json!({
            "target_type": "blob",
            "repository": format!("{owner}/{repo}"),
            "branch": branch,
            "path": path,
            "content": self.truncate_chars(content.trim(), self.max_result_chars.saturating_sub(200)),
        }))
    }

    async fn inspect_tree(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
        path: &str,
    ) -> Result<Value, String> {
        let api_url = if path.is_empty() {
            format!("https://api.github.com/repos/{owner}/{repo}/contents?ref={branch}")
        } else {
            format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={branch}")
        };
        let listing = self.fetch_json(&api_url).await?;
        let entries = listing
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                json!({
                    "name": item["name"].as_str().unwrap_or(""),
                    "path": item["path"].as_str().unwrap_or(""),
                    "type": item["type"].as_str().unwrap_or(""),
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "target_type": "tree",
            "repository": format!("{owner}/{repo}"),
            "branch": branch,
            "directory": path,
            "entries": entries,
        }))
    }
}

#[async_trait]
impl Tool for GitHubRepoInspectTool {
    fn name(&self) -> &str {
        "github_repo_inspect"
    }

    fn description(&self) -> &str {
        "Inspect an external GitHub repository, file, or directory and return structured technical architecture signals. Args: {\"url\": \"https://github.com/org/repo\"}."
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(
            ToolScope::GitHub,
            vec![ToolIntent::InspectRepository, ToolIntent::FetchWebPage],
        )
        .with_risk(ToolRisk::Low)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_freshness(ToolFreshness::BestEffort)
        .with_preferred_use_cases(vec![ToolUseCase::ExternalGitHubRepo])
        .with_disallowed_use_cases(vec![ToolUseCase::TimeSensitiveCurrent])
        .with_capability(
            ToolCapabilityGroup::RepositoryAnalysis,
            ToolCapabilitySubgroup::GitHubRepo,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
        )
        .with_preferred_rank(1)
        .with_critic_mode(ToolCriticMode::Conservative)
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let url = args
            .get("url")
            .or_else(|| args.get("repo_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if url.is_empty() {
            return Err("Missing url".to_string());
        }

        let result = match Self::parse_target(url) {
            Some(GitHubTarget::Repo { owner, repo }) => self.inspect_repo(&owner, &repo).await?,
            Some(GitHubTarget::Blob {
                owner,
                repo,
                branch,
                path,
            }) => self.inspect_blob(&owner, &repo, &branch, &path).await?,
            Some(GitHubTarget::Tree {
                owner,
                repo,
                branch,
                path,
            }) => self.inspect_tree(&owner, &repo, &branch, &path).await?,
            None => return Err("Invalid GitHub repository URL".to_string()),
        };

        let summary = match result["target_type"].as_str().unwrap_or("") {
            "repo" => result["repo_summary"]
                .as_str()
                .unwrap_or("Inspected GitHub repository")
                .to_string(),
            "blob" => format!(
                "Fetched GitHub file {}",
                result["path"].as_str().unwrap_or("")
            ),
            "tree" => format!(
                "Listed GitHub directory {}",
                result["directory"].as_str().unwrap_or("")
            ),
            _ => "Inspected GitHub target".to_string(),
        };

        let wrapped = output::structured(self.name(), summary, true, result)?;
        Ok(self.truncate_chars(&wrapped, self.max_result_chars))
    }
}
