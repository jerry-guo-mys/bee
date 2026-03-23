//! 代码 smell 检测模块

use super::analyzer::{Issue, Severity};

/// 代码 smell 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmellType {
    LongMethod,
    LongParameterList,
    GodClass,
    FeatureEnvy,
    DataClass,
    DuplicateCode,
    LongLine,
    TooManyVariables,
    NestedBlocks,
    MagicNumber,
}

impl std::fmt::Display for SmellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmellType::LongMethod => write!(f, "Long Method"),
            SmellType::LongParameterList => write!(f, "Long Parameter List"),
            SmellType::GodClass => write!(f, "God Class"),
            SmellType::FeatureEnvy => write!(f, "Feature Envy"),
            SmellType::DataClass => write!(f, "Data Class"),
            SmellType::DuplicateCode => write!(f, "Duplicate Code"),
            SmellType::LongLine => write!(f, "Long Line"),
            SmellType::TooManyVariables => write!(f, "Too Many Variables"),
            SmellType::NestedBlocks => write!(f, "Nested Blocks"),
            SmellType::MagicNumber => write!(f, "Magic Number"),
        }
    }
}

/// Smell 检测器
pub struct SmellDetector {
    config: SmellDetectorConfig,
}

#[derive(Debug, Clone)]
pub struct SmellDetectorConfig {
    pub max_method_length: usize,
    pub max_parameters: usize,
    pub max_class_methods: usize,
    pub max_line_length: usize,
    pub max_variables: usize,
    pub max_nesting_depth: usize,
    pub detect_magic_numbers: bool,
}

impl Default for SmellDetectorConfig {
    fn default() -> Self {
        Self {
            max_method_length: 50,
            max_parameters: 5,
            max_class_methods: 20,
            max_line_length: 120,
            max_variables: 10,
            max_nesting_depth: 4,
            detect_magic_numbers: true,
        }
    }
}

impl SmellDetector {
    pub fn new(config: SmellDetectorConfig) -> Self {
        Self { config }
    }

    pub fn detect(&self, content: &str, language: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        issues.extend(self.detect_long_lines(content));
        issues.extend(self.detect_nested_blocks(content, language));
        issues.extend(self.detect_long_methods(content, language));
        issues.extend(self.detect_magic_numbers(content, language));

        issues
    }

    fn detect_long_lines(&self, content: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if line.len() > self.config.max_line_length {
                issues.push(Issue {
                    line: line_num + 1,
                    column: self.config.max_line_length + 1,
                    severity: Severity::Info,
                    category: "style".to_string(),
                    message: format!(
                        "行长度 {} 超过 {} 字符限制",
                        line.len(),
                        self.config.max_line_length
                    ),
                    suggestion: Some("考虑将长行拆分为多行".to_string()),
                });
            }
        }

        issues
    }

    fn detect_nested_blocks(&self, content: &str, language: &str) -> Vec<Issue> {
        let mut issues = Vec::new();
        let mut current_depth = 0;
        let mut max_depth = 0;
        let mut max_depth_line = 1;

        for (line_num, line) in content.lines().enumerate() {
            // 计算缩进深度（简化实现）
            let indent_chars = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
            let depth = indent_chars / 4; // 假设 4 空格缩进

            if depth > self.config.max_nesting_depth {
                if depth > max_depth {
                    max_depth = depth;
                    max_depth_line = line_num + 1;
                }
            }
        }

        if max_depth > self.config.max_nesting_depth {
            issues.push(Issue {
                line: max_depth_line,
                column: 1,
                severity: Severity::Warning,
                category: "design".to_string(),
                message: format!(
                    "嵌套深度 {} 超过 {} 层限制",
                    max_depth,
                    self.config.max_nesting_depth
                ),
                suggestion: Some("考虑使用提前返回或提取函数来减少嵌套".to_string()),
            });
        }

        issues
    }

    fn detect_long_methods(&self, content: &str, language: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // 简化的方法长度检测
        let func_keyword = match language {
            "rust" => "fn ",
            "typescript" | "javascript" => "function ",
            "python" => "def ",
            _ => "fn ",
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut in_function = false;
        let mut function_start = 0;
        let mut function_length = 0;

        for (line_num, line) in lines.iter().enumerate() {
            if line.contains(func_keyword) && !in_function {
                in_function = true;
                function_start = line_num;
                function_length = 1;
            } else if in_function {
                function_length += 1;
                // 简化：检测函数结束
                if line.trim() == "}" || line.starts_with("}") ||
                   (language == "python" && !line.starts_with(' ') && !line.trim().is_empty() && !line.starts_with('#'))
                {
                    if function_length > self.config.max_method_length {
                        issues.push(Issue {
                            line: function_start + 1,
                            column: 1,
                            severity: Severity::Warning,
                            category: "design".to_string(),
                            message: format!(
                                "方法长度 {} 行超过 {} 行限制",
                                function_length,
                                self.config.max_method_length
                            ),
                            suggestion: Some("考虑将方法拆分为更小的方法".to_string()),
                        });
                    }
                    in_function = false;
                }
            }
        }

        issues
    }

    fn detect_magic_numbers(&self, content: &str, language: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        if !self.config.detect_magic_numbers {
            return issues;
        }

        // 简化的 magic number 检测
        // 检测代码中直接出现的数字（排除 0, 1, -1 等常见值）
        let number_pattern = regex::Regex::new(r"\b\d+\b").unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            // 跳过常量和配置定义
            if line.contains("const ") || line.contains("CONFIG") || line.contains("MAX_") || line.contains("MIN_") {
                continue;
            }

            for cap in number_pattern.find_iter(line) {
                let num_str = cap.as_str();
                if let Ok(num) = num_str.parse::<i32>() {
                    // 排除常见的合法数字
                    if ![0, 1, -1, 2, 10].contains(&num) {
                        // 检查是否是数组索引或循环边界
                        let context = &line[cap.start().saturating_sub(10)..cap.end()];
                        if !context.contains("for") && !context.contains("while") && !context.contains("[") {
                            issues.push(Issue {
                                line: line_num + 1,
                                column: cap.start() + 1,
                                severity: Severity::Info,
                                category: "code-smell".to_string(),
                                message: format!("发现魔法数字: {}", num),
                                suggestion: Some("考虑将其定义为具名常量".to_string()),
                            });
                            break; // 每行只报告一次
                        }
                    }
                }
            }
        }

        issues
    }
}

// 如果没有 regex 依赖，提供一个简单的实现
impl Default for SmellDetector {
    fn default() -> Self {
        Self::new(SmellDetectorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_line_detection() {
        let detector = SmellDetector::default();
        let content = "// This is a very long comment line that exceeds the maximum line length limit of 120 characters because it keeps going and going without stopping";

        let issues = detector.detect(content, "rust");
        assert!(!issues.is_empty());
        assert_eq!(issues[0].category, "style");
    }

    #[test]
    fn test_nested_blocks_detection() {
        let detector = SmellDetector::new(SmellDetectorConfig {
            max_nesting_depth: 2,
            ..Default::default()
        });

        let content = r#"
fn main() {
    if true {
        if true {
            if true {
                println!("deep");
            }
        }
    }
}
"#;

        let issues = detector.detect(content, "rust");
        assert!(!issues.is_empty());
        assert_eq!(issues[0].category, "design");
    }
}
