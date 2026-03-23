//! 复杂度分析模块

/// 计算循环复杂度
pub fn calculate_cyclomatic_complexity(code: &str, language: &str) -> usize {
    let mut complexity = 1; // 基础复杂度

    let keywords = match language {
        "rust" => vec!["if", "else", "for", "while", "match", "?", "&&", "||"],
        "typescript" | "javascript" => vec!["if", "else", "for", "while", "switch", "case", "?", "&&", "||"],
        "python" => vec!["if", "elif", "else", "for", "while", "and", "or", "except"],
        _ => vec![],
    };

    for keyword in keywords {
        complexity += code.matches(keyword).count();
    }

    complexity
}

/// 计算认知复杂度
pub fn calculate_cognitive_complexity(code: &str, language: &str) -> usize {
    let mut complexity = 0;
    let mut nesting_level = 0;

    let lines: Vec<&str> = code.lines().collect();
    for line in lines {
        let trimmed = line.trim();

        // 检测控制流结构
        if is_control_flow_start(trimmed, language) {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if is_control_flow_end(trimmed, language) {
            nesting_level = nesting_level.saturating_sub(1);
        } else if is_logical_operator(trimmed) {
            complexity += 1;
        }
    }

    complexity
}

fn is_control_flow_start(line: &str, language: &str) -> bool {
    match language {
        "rust" => {
            line.starts_with("if ") || line.starts_with("for ") || line.starts_with("while ") ||
            line.starts_with("match ")
        }
        "typescript" | "javascript" => {
            line.starts_with("if (") || line.starts_with("for (") || line.starts_with("while (") ||
            line.starts_with("switch (") || line.starts_with("try {")
        }
        "python" => {
            line.starts_with("if ") || line.starts_with("for ") || line.starts_with("while ") ||
            line.starts_with("elif ") || line.starts_with("except") || line.starts_with("with ")
        }
        _ => false,
    }
}

fn is_control_flow_end(_line: &str, _language: &str) -> bool {
    // 简化实现
    false
}

fn is_logical_operator(line: &str) -> bool {
    line.contains("&&") || line.contains("||")
}

/// 复杂度评级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityRating {
    Low,       // 1-5: 简单
    Medium,    // 6-10: 中等
    High,      // 11-20: 复杂
    VeryHigh,  // 21+: 非常复杂
}

impl From<usize> for ComplexityRating {
    fn from(value: usize) -> Self {
        match value {
            0..=5 => ComplexityRating::Low,
            6..=10 => ComplexityRating::Medium,
            11..=20 => ComplexityRating::High,
            _ => ComplexityRating::VeryHigh,
        }
    }
}

impl std::fmt::Display for ComplexityRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplexityRating::Low => write!(f, "Low (简单)"),
            ComplexityRating::Medium => write!(f, "Medium (中等)"),
            ComplexityRating::High => write!(f, "High (复杂)"),
            ComplexityRating::VeryHigh => write!(f, "Very High (非常复杂)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclomatic_simple() {
        let code = r#"
        if x > 0 {
            for i in 0..10 {
                if i % 2 == 0 {
                    continue;
                }
            }
        }
        "#;

        let complexity = calculate_cyclomatic_complexity(code, "rust");
        assert!(complexity >= 4); // if + for + if = 3 + base 1
    }

    #[test]
    fn test_complexity_rating() {
        assert_eq!(ComplexityRating::from(3), ComplexityRating::Low);
        assert_eq!(ComplexityRating::from(8), ComplexityRating::Medium);
        assert_eq!(ComplexityRating::from(15), ComplexityRating::High);
        assert_eq!(ComplexityRating::from(25), ComplexityRating::VeryHigh);
    }
}
