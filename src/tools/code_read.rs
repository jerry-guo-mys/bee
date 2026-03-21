//! 代码读取工具 - 安全地读取项目代码文件
//!
//! 用于自主迭代时读取代码内容进行分析

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::Value;

use crate::tools::Tool;

/// 代码读取工具
pub struct CodeReadTool {
    /// 允许的根目录（通常是项目根目录）
    allowed_root: PathBuf,
    /// 回退根目录（通常是当前仓库根）
    fallback_root: Option<PathBuf>,
    /// 最大读取行数
    max_lines: usize,
    /// 单行最大字符数
    max_line_length: usize,
}

impl CodeReadTool {
    pub fn new(allowed_root: impl AsRef<Path>) -> Self {
        let root = allowed_root.as_ref().to_path_buf();
        let allowed_root = root.canonicalize().unwrap_or(root);
        let fallback_root = std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.canonicalize().ok().or(Some(cwd)))
            .filter(|cwd| cwd != &allowed_root);

        Self {
            allowed_root,
            fallback_root,
            max_lines: 2000,
            max_line_length: 2000,
        }
    }

    pub fn with_limits(mut self, max_lines: usize, max_line_length: usize) -> Self {
        self.max_lines = max_lines;
        self.max_line_length = max_line_length;
        self
    }

    /// 验证路径是否在允许范围内
    fn validate_under_root(root: &Path, file_path: &str) -> Result<PathBuf, String> {
        let trimmed = file_path.trim_start_matches("./");
        let candidate = root.join(trimmed);
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let canonical_path = candidate
            .canonicalize()
            .map_err(|_| format!("File not found: {}", candidate.display()))?;

        if canonical_path.starts_with(&canonical_root) {
            Ok(canonical_path)
        } else {
            Err(format!(
                "Access denied: path '{}' is outside allowed root '{}'",
                file_path,
                root.display()
            ))
        }
    }

    /// 验证路径是否在允许范围内，允许回退到当前仓库根
    fn validate_path(&self, file_path: &str) -> Result<PathBuf, String> {
        match Self::validate_under_root(&self.allowed_root, file_path) {
            Ok(path) => Ok(path),
            Err(err) if err.starts_with("File not found:") => {
                if let Some(fallback_root) = &self.fallback_root {
                    Self::validate_under_root(fallback_root, file_path)
                } else {
                    Err(err)
                }
            }
            Err(err) => Err(err),
        }
    }

    /// 读取文件内容（带行号）
    fn read_file_with_lines(
        &self,
        file_path: &Path,
        offset: usize,
        limit: Option<usize>,
    ) -> Result<String, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if offset >= total_lines {
            return Ok(format!(
                "File '{}' has {} lines. Requested offset {} is beyond end.",
                file_path.display(),
                total_lines,
                offset
            ));
        }

        let end = limit
            .map(|l| (offset + l).min(total_lines))
            .unwrap_or(total_lines);
        let slice = &lines[offset..end];

        let mut result = String::new();
        result.push_str(&format!(
            "File: {} (lines {}-{} of {})\n",
            file_path.display(),
            offset + 1,
            end,
            total_lines
        ));
        result.push_str(&"-".repeat(60));
        result.push('\n');

        for (i, line) in slice.iter().enumerate() {
            let line_num = offset + i + 1;
            let truncated = if line.len() > self.max_line_length {
                format!("{}...", &line[..self.max_line_length])
            } else {
                line.to_string()
            };
            result.push_str(&format!("{:4}: {}\n", line_num, truncated));
        }

        if end < total_lines {
            result.push_str(&format!(
                "\n... ({} more lines, use offset={} to continue)\n",
                total_lines - end,
                end
            ));
        }

        Ok(result)
    }
}

#[async_trait]
impl Tool for CodeReadTool {
    fn name(&self) -> &str {
        "code_read"
    }

    fn description(&self) -> &str {
        "Read code file contents with line numbers"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "File path (relative to project root or absolute)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Starting line number (1-based, default: 1)",
                    "default": 1
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum lines to read (default: 200)",
                    "default": 200
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, String> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: file_path")?;

        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1)
            .saturating_sub(1); // 转换为0-based

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .or(Some(200));

        let validated_path = self.validate_path(file_path)?;

        if !validated_path.exists() {
            return Err(format!("File not found: {}", validated_path.display()));
        }

        if !validated_path.is_file() {
            return Err(format!("Path is not a file: {}", validated_path.display()));
        }

        self.read_file_with_lines(&validated_path, offset, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_security() {
        let test_dir = std::path::PathBuf::from("./target/test_code_read");
        std::fs::create_dir_all(&test_dir).unwrap();
        std::fs::create_dir_all(test_dir.join("src")).unwrap();
        std::fs::write(test_dir.join("Cargo.toml"), "").unwrap();
        std::fs::write(test_dir.join("src/main.rs"), "fn main() {}").unwrap();

        let tool = CodeReadTool::new(&test_dir);

        // 正常路径
        assert!(tool.validate_path("src/main.rs").is_ok());
        assert!(tool.validate_path("Cargo.toml").is_ok());

        // 路径穿越攻击应该被阻止
        assert!(tool.validate_path("../../../etc/passwd").is_err());
        assert!(tool.validate_path("src/../../../etc/passwd").is_err());

        std::fs::remove_dir_all(&test_dir).ok();
    }

    #[test]
    fn test_read_nonexistent_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tool = CodeReadTool::new(".");

        rt.block_on(async {
            let args = serde_json::json!({
                "file_path": "nonexistent_file_xyz.txt"
            });
            let result = tool.execute(args).await;
            assert!(result.is_err());
        });
    }
}
