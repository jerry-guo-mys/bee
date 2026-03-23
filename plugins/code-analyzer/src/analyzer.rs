//! 代码分析器核心实现

use std::collections::HashMap;
use std::path::Path;

/// 分析结果
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub file_path: String,
    pub language: String,
    pub metrics: CodeMetrics,
    pub issues: Vec<Issue>,
}

/// 代码指标
#[derive(Debug, Clone, Default)]
pub struct CodeMetrics {
    pub lines_of_code: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub cyclomatic_complexity: usize,
    pub cognitive_complexity: usize,
    pub halstead_metrics: HalsteadMetrics,
}

/// Halstead 复杂度指标
#[derive(Debug, Clone, Default)]
pub struct HalsteadMetrics {
    pub n1: usize, // 不同操作符数量
    pub n2: usize, // 不同操作数数量
    pub N1: usize, // 总操作符数
    pub N2: usize, // 总操作数
    pub vocabulary: usize,
    pub length: usize,
    pub volume: f64,
    pub difficulty: f64,
    pub effort: f64,
}

/// 代码问题
#[derive(Debug, Clone)]
pub struct Issue {
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// 问题严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 代码分析器
pub struct CodeAnalyzer {
    config: AnalyzerConfig,
    language_parsers: HashMap<String, Box<dyn LanguageParser>>,
}

#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub max_file_size_kb: usize,
    pub supported_languages: Vec<String>,
    pub enable_complexity_check: bool,
    pub enable_smell_check: bool,
    pub complexity_threshold: usize,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            max_file_size_kb: 1024,
            supported_languages: vec![
                "rust".to_string(),
                "typescript".to_string(),
                "javascript".to_string(),
                "python".to_string(),
            ],
            enable_complexity_check: true,
            enable_smell_check: true,
            complexity_threshold: 10,
        }
    }
}

trait LanguageParser: Send + Sync {
    fn parse(&self, content: &str) -> Result<AnalysisResult, String>;
    fn detect_language(&self, path: &Path) -> Option<String>;
}

impl CodeAnalyzer {
    pub fn new(config: AnalyzerConfig) -> Self {
        Self {
            config,
            language_parsers: HashMap::new(),
        }
    }

    pub fn register_parser(&mut self, language: String, parser: Box<dyn LanguageParser>) {
        self.language_parsers.insert(language, parser);
    }

    pub fn analyze_file(&self, path: &Path) -> Result<AnalysisResult, AnalyzerError> {
        // 检查文件大小
        let metadata = std::fs::metadata(path)
            .map_err(|e| AnalyzerError::IoError(path.to_path_buf(), e))?;

        let file_size_kb = metadata.len() / 1024;
        if file_size_kb > self.config.max_file_size_kb as u64 {
            return Err(AnalyzerError::FileTooLarge {
                path: path.to_path_buf(),
                size_kb: file_size_kb,
                max_kb: self.config.max_file_size_kb,
            });
        }

        // 读取文件内容
        let content = std::fs::read_to_string(path)
            .map_err(|e| AnalyzerError::IoError(path.to_path_buf(), e))?;

        // 检测语言
        let language = self.detect_language(path);
        if !self.config.supported_languages.contains(&language) {
            return Err(AnalyzerError::UnsupportedLanguage {
                language,
                supported: self.config.supported_languages.clone(),
            });
        }

        // 分析文件
        self.analyze_content(&content, &language)
    }

    pub fn analyze_content(&self, content: &str, language: &str) -> Result<AnalysisResult, AnalyzerError> {
        let mut metrics = CodeMetrics::default();
        let mut issues = Vec::new();

        // 计算基础指标
        let lines: Vec<&str> = content.lines().collect();
        metrics.lines_of_code = lines.len();
        metrics.blank_lines = lines.iter().filter(|l| l.trim().is_empty()).count();
        metrics.comment_lines = lines.iter()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*")
            })
            .count();

        // 复杂度分析
        if self.config.enable_complexity_check {
            let (cyclomatic, cognitive) = self.analyze_complexity(content, language);
            metrics.cyclomatic_complexity = cyclomatic;
            metrics.cognitive_complexity = cognitive;

            // 检查复杂度阈值
            if cyclomatic > self.config.complexity_threshold {
                issues.push(Issue {
                    line: 1,
                    column: 0,
                    severity: Severity::Warning,
                    category: "complexity".to_string(),
                    message: format!("循环复杂度 {} 超过阈值 {}", cyclomatic, self.config.complexity_threshold),
                    suggestion: Some("考虑将函数拆分为更小的函数".to_string()),
                });
            }
        }

        // Smell 检测
        if self.config.enable_smell_check {
            issues.extend(self.detect_smells(content, language));
        }

        Ok(AnalysisResult {
            file_path: String::new(),
            language: language.to_string(),
            metrics,
            issues,
        })
    }

    fn analyze_complexity(&self, content: &str, language: &str) -> (usize, usize) {
        // 简化的循环复杂度计算
        let mut cyclomatic = 1; // 基础复杂度

        // 统计控制流关键字
        let keywords = match language {
            "rust" | "typescript" | "javascript" => {
                vec!["if", "else", "for", "while", "match", "case", "?", "&&", "||"]
            }
            "python" => {
                vec!["if", "elif", "else", "for", "while", "and", "or", "except"]
            }
            _ => vec![],
        };

        for keyword in keywords {
            cyclomatic += content.matches(keyword).count();
        }

        // 简化的认知复杂度计算
        let cognitive = cyclomatic + content.matches('{').count() / 2;

        (cyclomatic, cognitive)
    }

    fn detect_smells(&self, content: &str, _language: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // 检测长行
        for (line_num, line) in content.lines().enumerate() {
            if line.len() > 120 {
                issues.push(Issue {
                    line: line_num + 1,
                    column: 121,
                    severity: Severity::Info,
                    category: "style".to_string(),
                    message: format!("行长度 {} 超过 120 字符", line.len()),
                    suggestion: Some("考虑将长行拆分为多行".to_string()),
                });
            }
        }

        // 检测过长的函数（简化的检测）
        let func_count = content.matches("fn ").count() + content.matches("function ").count();
        if func_count > 0 && content.lines().count() / func_count > 50 {
            issues.push(Issue {
                line: 1,
                column: 0,
                severity: Severity::Warning,
                category: "design".to_string(),
                message: "函数平均长度过长".to_string(),
                suggestion: Some("考虑将大函数拆分为小函数".to_string()),
            });
        }

        issues
    }

    fn detect_language(&self, path: &Path) -> String {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("ts") | Some("tsx") => "typescript".to_string(),
            Some("js") | Some("jsx") => "javascript".to_string(),
            Some("py") => "python".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AnalyzerError {
    #[error("IO error for path {0}: {1}")]
    IoError(std::path::PathBuf, std::io::Error),

    #[error("File too large: {path:?} ({size_kb}KB > max {max_kb}KB)")]
    FileTooLarge {
        path: std::path::PathBuf,
        size_kb: u64,
        max_kb: usize,
    },

    #[error("Unsupported language: {language}, supported: {supported:?}")]
    UnsupportedLanguage {
        language: String,
        supported: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_metrics() {
        let analyzer = CodeAnalyzer::new(AnalyzerConfig::default());
        let content = r#"
// This is a comment
fn main() {
    let x = 1;
    if x > 0 {
        println!("positive");
    }
}
"#;

        let result = analyzer.analyze_content(content, "rust").unwrap();
        assert!(result.metrics.lines_of_code > 0);
        assert!(result.metrics.comment_lines > 0);
    }

    #[test]
    fn test_complexity_detection() {
        let analyzer = CodeAnalyzer::new(AnalyzerConfig {
            complexity_threshold: 5,
            ..Default::default()
        });

        let content = r#"
fn complex_function() {
    if true {
        if true {
            for i in 0..10 {
                while i < 5 {
                    match i {
                        _ => {}
                    }
                }
            }
        }
    }
}
"#;

        let result = analyzer.analyze_content(content, "rust").unwrap();
        assert!(!result.issues.is_empty());
    }
}
