//! 测试策略模块

use super::generator::{TestCase, TestInput, TestType, FunctionInfo, ParamInfo};

/// 测试策略 trait
pub trait TestStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn generate_tests(&self, functions: &[FunctionInfo]) -> Vec<TestCase>;
}

/// 单元测试策略
pub struct UnitTestStrategy;

impl TestStrategy for UnitTestStrategy {
    fn name(&self) -> &str {
        "Unit Test"
    }

    fn generate_tests(&self, functions: &[FunctionInfo]) -> Vec<TestCase> {
        functions.iter().map(|func| {
            TestCase {
                name: format!("test_{}_unit", func.name),
                description: format!("Unit test for {}", func.name),
                inputs: generate_default_inputs(&func.params),
                expected_output: "expected_value".to_string(),
                test_type: TestType::Unit,
            }
        }).collect()
    }
}

/// 边界测试策略
pub struct BoundaryTestStrategy;

impl TestStrategy for BoundaryTestStrategy {
    fn name(&self) -> &str {
        "Boundary Test"
    }

    fn generate_tests(&self, functions: &[FunctionInfo]) -> Vec<TestCase> {
        let mut tests = Vec::new();

        for func in functions {
            // 数值类型的边界测试
            if has_numeric_param(&func.params) {
                tests.push(TestCase {
                    name: format!("test_{}_min_value", func.name),
                    description: format!("Test {} with minimum value", func.name),
                    inputs: generate_boundary_inputs(&func.params, Boundary::Min),
                    expected_output: "expected_value".to_string(),
                    test_type: TestType::EdgeCase,
                });

                tests.push(TestCase {
                    name: format!("test_{}_max_value", func.name),
                    description: format!("Test {} with maximum value", func.name),
                    inputs: generate_boundary_inputs(&func.params, Boundary::Max),
                    expected_output: "expected_value".to_string(),
                    test_type: TestType::EdgeCase,
                });
            }

            // 空值测试
            if has_optional_param(&func.params) {
                tests.push(TestCase {
                    name: format!("test_{}_none", func.name),
                    description: format!("Test {} with None input", func.name),
                    inputs: generate_empty_inputs(&func.params),
                    expected_output: "expected_value".to_string(),
                    test_type: TestType::EdgeCase,
                });
            }
        }

        tests
    }
}

/// 属性测试策略
pub struct PropertyTestStrategy;

impl TestStrategy for PropertyTestStrategy {
    fn name(&self) -> &str {
        "Property Test"
    }

    fn generate_tests(&self, functions: &[FunctionInfo]) -> Vec<TestCase> {
        functions.iter().map(|func| {
            TestCase {
                name: format!("test_{}_idempotent", func.name),
                description: format!("Property test: {} should be idempotent", func.name),
                inputs: generate_random_inputs(&func.params),
                expected_output: "f(f(x)) == f(x)".to_string(),
                test_type: TestType::Property,
            }
        }).collect()
    }
}

/// 模糊测试策略
pub struct FuzzTestStrategy;

impl TestStrategy for FuzzTestStrategy {
    fn name(&self) -> &str {
        "Fuzz Test"
    }

    fn generate_tests(&self, functions: &[FunctionInfo]) -> Vec<TestCase> {
        functions.iter().map(|func| {
            TestCase {
                name: format!("test_{}_fuzz", func.name),
                description: format!("Fuzz test for {}", func.name),
                inputs: vec![TestInput {
                    name: "data".to_string(),
                    value: "arbitrary()".to_string(),
                    input_type: "Bytes".to_string(),
                }],
                expected_output: "no_panic()".to_string(),
                test_type: TestType::EdgeCase,
            }
        }).collect()
    }
}

fn has_numeric_param(params: &[ParamInfo]) -> bool {
    params.iter().any(|p| {
        p.type_name.contains("i8") || p.type_name.contains("i16") ||
        p.type_name.contains("i32") || p.type_name.contains("i64") ||
        p.type_name.contains("i128") || p.type_name.contains("isize") ||
        p.type_name.contains("u8") || p.type_name.contains("u16") ||
        p.type_name.contains("u32") || p.type_name.contains("u64") ||
        p.type_name.contains("u128") || p.type_name.contains("usize") ||
        p.type_name.contains("f32") || p.type_name.contains("f64")
    })
}

fn has_optional_param(params: &[ParamInfo]) -> bool {
    params.iter().any(|p| {
        p.type_name.starts_with("Option<") ||
        p.type_name == "&str" ||
        p.type_name == "String"
    })
}

fn generate_default_inputs(params: &[ParamInfo]) -> Vec<TestInput> {
    params.iter().map(|p| {
        let value = match p.type_name.as_str() {
            "i32" | "i64" | "i16" | "i8" => "42".to_string(),
            "u32" | "u64" | "u16" | "u8" | "usize" => "1".to_string(),
            "f32" | "f64" => "3.14".to_string(),
            "bool" => "true".to_string(),
            "String" => "\"test\".to_string()".to_string(),
            "&str" => "\"test\"".to_string(),
            t if t.starts_with("Option<") => "Some(Default::default())".to_string(),
            _ => "Default::default()".to_string(),
        };
        TestInput {
            name: p.name.clone(),
            value,
            input_type: p.type_name.clone(),
        }
    }).collect()
}

fn generate_boundary_inputs(params: &[ParamInfo], boundary: Boundary) -> Vec<TestInput> {
    params.iter().map(|p| {
        let value = match p.type_name.as_str() {
            "i32" => match boundary {
                Boundary::Min => "i32::MIN".to_string(),
                Boundary::Max => "i32::MAX".to_string(),
            },
            "i64" => match boundary {
                Boundary::Min => "i64::MIN".to_string(),
                Boundary::Max => "i64::MAX".to_string(),
            },
            "u32" => match boundary {
                Boundary::Min => "0".to_string(),
                Boundary::Max => "u32::MAX".to_string(),
            },
            "usize" => match boundary {
                Boundary::Min => "0".to_string(),
                Boundary::Max => "usize::MAX".to_string(),
            },
            "f32" | "f64" => match boundary {
                Boundary::Min => "f64::NEG_INFINITY".to_string(),
                Boundary::Max => "f64::INFINITY".to_string(),
            },
            _ => "Default::default()".to_string(),
        };
        TestInput {
            name: p.name.clone(),
            value,
            input_type: p.type_name.clone(),
        }
    }).collect()
}

fn generate_empty_inputs(params: &[ParamInfo]) -> Vec<TestInput> {
    params.iter().map(|p| {
        let value = match p.type_name.as_str() {
            "String" => "String::new()".to_string(),
            "&str" => "\"\"".to_string(),
            t if t.starts_with("Vec<") => "Vec::new()".to_string(),
            t if t.starts_with("Option<") => "None".to_string(),
            t if t.starts_with("HashMap<") => "HashMap::new()".to_string(),
            _ => "Default::default()".to_string(),
        };
        TestInput {
            name: p.name.clone(),
            value,
            input_type: p.type_name.clone(),
        }
    }).collect()
}

fn generate_random_inputs(params: &[ParamInfo]) -> Vec<TestInput> {
    params.iter().map(|p| {
        let value = match p.type_name.as_str() {
            "i32" | "i64" | "usize" => "arbitrary()".to_string(),
            "String" => "arbitrary()".to_string(),
            "bool" => "arbitrary()".to_string(),
            _ => "arbitrary()".to_string(),
        };
        TestInput {
            name: p.name.clone(),
            value,
            input_type: p.type_name.clone(),
        }
    }).collect()
}

#[derive(Debug, Clone, Copy)]
enum Boundary {
    Min,
    Max,
}

/// 测试策略组合器
pub struct TestStrategyComposer {
    strategies: Vec<Box<dyn TestStrategy>>,
}

impl TestStrategyComposer {
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: Box<dyn TestStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn TestStrategy>) {
        self.strategies.push(strategy);
    }

    pub fn generate_all_tests(&self, functions: &[FunctionInfo]) -> Vec<TestCase> {
        let mut all_tests = Vec::new();
        for strategy in &self.strategies {
            all_tests.extend(strategy.generate_tests(functions));
        }
        all_tests
    }
}

impl Default for TestStrategyComposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_strategy() {
        let strategy = UnitTestStrategy;
        let functions = vec![FunctionInfo {
            name: "add".to_string(),
            params: vec![
                ParamInfo { name: "a".to_string(), type_name: "i32".to_string() },
                ParamInfo { name: "b".to_string(), type_name: "i32".to_string() },
            ],
            return_type: "i32".to_string(),
        }];

        let tests = strategy.generate_tests(&functions);
        assert_eq!(tests.len(), 1);
        assert!(tests[0].name.starts_with("test_add_unit"));
    }

    #[test]
    fn test_boundary_strategy() {
        let strategy = BoundaryTestStrategy;
        let functions = vec![FunctionInfo {
            name: "process".to_string(),
            params: vec![
                ParamInfo { name: "x".to_string(), type_name: "i32".to_string() },
            ],
            return_type: "i32".to_string(),
        }];

        let tests = strategy.generate_tests(&functions);
        assert_eq!(tests.len(), 2); // min and max
    }

    #[test]
    fn test_strategy_composer() {
        let mut composer = TestStrategyComposer::new();
        composer.add_strategy(Box::new(UnitTestStrategy));
        composer.add_strategy(Box::new(BoundaryTestStrategy));

        let functions = vec![FunctionInfo {
            name: "compute".to_string(),
            params: vec![
                ParamInfo { name: "n".to_string(), type_name: "i32".to_string() },
            ],
            return_type: "i32".to_string(),
        }];

        let tests = composer.generate_all_tests(&functions);
        assert_eq!(tests.len(), 3); // 1 unit + 2 boundary
    }
}
