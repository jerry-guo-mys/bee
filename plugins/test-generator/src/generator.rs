//! 测试生成器核心实现

use std::path::Path;
use std::fs;

/// 测试生成器配置
#[derive(Debug, Clone)]
pub struct TestGeneratorConfig {
    pub test_framework: TestFramework,
    pub coverage_target: u8,
    pub generate_integration_tests: bool,
    pub test_output_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFramework {
    CargoTest,
    Rstest,
    Proptest,
    Quickcheck,
}

impl Default for TestGeneratorConfig {
    fn default() -> Self {
        Self {
            test_framework: TestFramework::CargoTest,
            coverage_target: 80,
            generate_integration_tests: true,
            test_output_dir: "./tests".to_string(),
        }
    }
}

/// 测试生成器
pub struct TestGenerator {
    config: TestGeneratorConfig,
}

/// 测试用例结构
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub inputs: Vec<TestInput>,
    pub expected_output: String,
    pub test_type: TestType,
}

#[derive(Debug, Clone)]
pub struct TestInput {
    pub name: String,
    pub value: String,
    pub input_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
    Unit,
    Integration,
    Property,
    EdgeCase,
}

impl TestGenerator {
    pub fn new(config: TestGeneratorConfig) -> Self {
        Self { config }
    }

    /// 从 Rust 源码生成测试
    pub fn generate_tests(&self, source_path: &Path) -> Result<String, TestGeneratorError> {
        let content = fs::read_to_string(source_path)
            .map_err(|e| TestGeneratorError::IoError(source_path.to_path_buf(), e))?;

        let functions = self.parse_functions(&content);
        let tests = self.generate_test_cases(&functions);

        Ok(self.render_tests(&tests))
    }

    /// 解析源码中的函数
    fn parse_functions(&self, content: &str) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // 解析 pub fn
            if trimmed.starts_with("pub fn ") {
                if let Some(func) = self.parse_function_line(trimmed, &lines, i) {
                    functions.push(func);
                }
            }
        }

        functions
    }

    fn parse_function_line(&self, line: &str, _lines: &[&str], _line_num: usize) -> Option<FunctionInfo> {
        // 提取函数名
        let name = line
            .strip_prefix("pub fn ")?
            .split('(')
            .next()?
            .trim()
            .to_string();

        // 提取参数
        let params = if let Some(start) = line.find('(') {
            if let Some(end) = line.find(')') {
                let params_str = &line[start + 1..end];
                self.parse_params(params_str)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // 提取返回类型
        let return_type = if let Some(ret_start) = line.find("-> ") {
            line[ret_start + 3..].trim().to_string()
        } else {
            "()".to_string()
        };

        Some(FunctionInfo {
            name,
            params,
            return_type,
        })
    }

    fn parse_params(&self, params_str: &str) -> Vec<ParamInfo> {
        let mut params = Vec::new();

        for param in params_str.split(',') {
            let param = param.trim();
            if param.is_empty() || param == "&self" || param == "&mut self" || param == "self" {
                continue;
            }

            if let Some(colon_pos) = param.find(':') {
                let name = param[..colon_pos].trim().to_string();
                let type_name = param[colon_pos + 1..].trim().to_string();
                params.push(ParamInfo { name, type_name });
            }
        }

        params
    }

    /// 为函数生成测试用例
    fn generate_test_cases(&self, functions: &[FunctionInfo]) -> Vec<TestCase> {
        let mut tests = Vec::new();

        for func in functions {
            // 生成基本单元测试
            tests.push(self.create_unit_test(func));

            // 生成边界测试
            tests.extend(self.create_edge_case_tests(func));

            // 如果配置了集成测试
            if self.config.generate_integration_tests {
                tests.push(self.create_integration_test(func));
            }
        }

        tests
    }

    fn create_unit_test(&self, func: &FunctionInfo) -> TestCase {
        TestCase {
            name: format!("test_{}_basic", func.name),
            description: format!("Basic unit test for {}", func.name),
            inputs: self.generate_default_inputs(&func.params),
            expected_output: self.infer_default_output(&func.return_type),
            test_type: TestType::Unit,
        }
    }

    fn create_edge_case_tests(&self, func: &FunctionInfo) -> Vec<TestCase> {
        let mut tests = Vec::new();

        // 空值测试
        if func.params.iter().any(|p| p.type_name.starts_with("Option<") || p.type_name.starts_with("&str")) {
            tests.push(TestCase {
                name: format!("test_{}_empty_input", func.name),
                description: format!("Test {} with empty input", func.name),
                inputs: self.generate_edge_case_inputs(&func.params, EdgeCase::Empty),
                expected_output: self.infer_default_output(&func.return_type),
                test_type: TestType::EdgeCase,
            });
        }

        // 边界值测试
        if func.params.iter().any(|p| p.type_name.contains("usize") || p.type_name.contains("i")) {
            tests.push(TestCase {
                name: format!("test_{}_boundary", func.name),
                description: format!("Test {} with boundary values", func.name),
                inputs: self.generate_edge_case_inputs(&func.params, EdgeCase::Boundary),
                expected_output: self.infer_default_output(&func.return_type),
                test_type: TestType::EdgeCase,
            });
        }

        tests
    }

    fn create_integration_test(&self, func: &FunctionInfo) -> TestCase {
        TestCase {
            name: format!("test_{}_integration", func.name),
            description: format!("Integration test for {}", func.name),
            inputs: self.generate_default_inputs(&func.params),
            expected_output: format!("// Integration test for {}", func.name),
            test_type: TestType::Integration,
        }
    }

    fn generate_default_inputs(&self, params: &[ParamInfo]) -> Vec<TestInput> {
        params.iter().map(|p| {
            let value = self.infer_default_value(&p.type_name);
            TestInput {
                name: p.name.clone(),
                value,
                input_type: p.type_name.clone(),
            }
        }).collect()
    }

    fn generate_edge_case_inputs(&self, params: &[ParamInfo], edge_case: EdgeCase) -> Vec<TestInput> {
        params.iter().map(|p| {
            let value = match edge_case {
                EdgeCase::Empty => self.get_empty_value(&p.type_name),
                EdgeCase::Boundary => self.get_boundary_value(&p.type_name),
            };
            TestInput {
                name: p.name.clone(),
                value,
                input_type: p.type_name.clone(),
            }
        }).collect()
    }

    fn infer_default_value(&self, type_name: &str) -> String {
        match type_name {
            t if t.starts_with("Option<") => "None".to_string(),
            t if t.starts_with("Result<") => "Ok(Default::default())".to_string(),
            t if t == "&str" || t == "String" => "\"test\".to_string()".to_string(),
            t if t == "usize" => "1".to_string(),
            t if t == "i32" || t == "i64" => "42".to_string(),
            t if t == "bool" => "true".to_string(),
            t if t == "()" => "()".to_string(),
            _ => "Default::default()".to_string(),
        }
    }

    fn get_empty_value(&self, type_name: &str) -> String {
        match type_name {
            t if t == "&str" => "\"\"".to_string(),
            t if t == "String" => "String::new()".to_string(),
            t if t.starts_with("Vec<") => "Vec::new()".to_string(),
            t if t.starts_with("Option<") => "None".to_string(),
            t if t.starts_with("HashMap<") => "HashMap::new()".to_string(),
            t if t == "usize" => "0".to_string(),
            t if t == "i32" || t == "i64" => "0".to_string(),
            _ => "Default::default()".to_string(),
        }
    }

    fn get_boundary_value(&self, type_name: &str) -> String {
        match type_name {
            t if t == "usize" => "usize::MAX".to_string(),
            t if t == "i32" => "i32::MAX".to_string(),
            t if t == "i64" => "i64::MAX".to_string(),
            _ => "Default::default()".to_string(),
        }
    }

    fn infer_default_output(&self, return_type: &str) -> String {
        match return_type {
            t if t.starts_with("Result<Ok," => "Ok(expected_value)".to_string(),
            t if t.starts_with("Option<" => "Some(expected_value)".to_string(),
            t if t == "bool" => "assert!(result)".to_string(),
            t if t == "()" => "// Function returns ()".to_string(),
            _ => "expected_value".to_string(),
        }
    }

    /// 渲染测试代码
    fn render_tests(&self, tests: &[TestCase]) -> String {
        let mut code = String::new();

        // 添加测试模块头
        code.push_str("#[cfg(test)]\nmod tests {\n");
        code.push_str("    use super::*;\n\n");

        match self.config.test_framework {
            TestFramework::CargoTest => {
                for test in tests {
                    code.push_str(&self.render_cargo_test(test));
                }
            }
            TestFramework::Rstest => {
                code.push_str("    use rstest::rstest;\n\n");
                for test in tests {
                    code.push_str(&self.render_rstest(test));
                }
            }
            _ => {
                for test in tests {
                    code.push_str(&self.render_cargo_test(test));
                }
            }
        }

        code.push_str("}\n");
        code
    }

    fn render_cargo_test(&self, test: &TestCase) -> String {
        let mut code = String::new();

        code.push_str(&format!("    /// {}\n", test.description));
        code.push_str("    #[test]\n");
        code.push_str(&format!("    fn {}() {{\n", test.name));

        // 生成输入
        for input in &test.inputs {
            code.push_str(&format!("        let {} = {};\n", input.name, input.value));
        }

        // 生成调用和断言
        let args: Vec<String> = test.inputs.iter().map(|i| i.name.clone()).collect();
        code.push_str(&format!("        let result = /* function_call */({});\n", args.join(", ")));
        code.push_str(&format!("        // TODO: Assert expected: {}\n", test.expected_output));
        code.push_str("    }\n\n");

        code
    }

    fn render_rstest(&self, test: &TestCase) -> String {
        let mut code = String::new();

        code.push_str(&format!("    /// {}\n", test.description));
        code.push_str("    #[rstest]\n");
        code.push_str(&format!("    fn {}(\n", test.name));

        // 生成参数
        for (i, input) in test.inputs.iter().enumerate() {
            let comma = if i < test.inputs.len() - 1 { "," } else { "" };
            code.push_str(&format!("        #[values({})] {}: {},\n", input.value, input.name, input.input_type));
        }

        code.push_str("    ) {\n");
        code.push_str("        // Test implementation\n");
        code.push_str("    }\n\n");

        code
    }

    /// 保存测试文件
    pub fn save_tests(&self, content: &str, test_name: &str) -> Result<(), TestGeneratorError> {
        fs::create_dir_all(&self.config.test_output_dir)
            .map_err(|e| TestGeneratorError::IoError(std::path::PathBuf::from(&self.config.test_output_dir), e))?;

        let path = Path::new(&self.config.test_output_dir).join(format!("{}.rs", test_name));
        fs::write(&path, content)
            .map_err(|e| TestGeneratorError::IoError(path, e))?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub params: Vec<ParamInfo>,
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Copy)]
enum EdgeCase {
    Empty,
    Boundary,
}

#[derive(Debug, thiserror::Error)]
pub enum TestGeneratorError {
    #[error("IO error for path {0}: {1}")]
    IoError(std::path::PathBuf, std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let generator = TestGenerator::new(TestGeneratorConfig::default());
        let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

        let functions = generator.parse_functions(source);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "add");
        assert_eq!(functions[0].params.len(), 2);
    }

    #[test]
    fn test_generate_test_code() {
        let generator = TestGenerator::new(TestGeneratorConfig::default());
        let source = r#"
pub fn multiply(x: usize, y: usize) -> usize {
    x * y
}
"#;

        let tests = generator.generate_tests(std::path::Path::new("test.rs")).unwrap();
        assert!(tests.contains("#[cfg(test)]"));
        assert!(tests.contains("#[test]"));
    }
}
