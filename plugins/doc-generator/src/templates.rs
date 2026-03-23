//! 文档模板模块

/// 文档模板
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub content: String,
}

/// 模板引擎
pub struct TemplateEngine {
    templates: Vec<Template>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    pub fn register_template(&mut self, name: String, content: String) {
        self.templates.push(Template { name, content });
    }

    pub fn render(&self, template_name: &str, data: &TemplateData) -> Result<String, TemplateError> {
        let template = self.templates
            .iter()
            .find(|t| t.name == template_name)
            .ok_or_else(|| TemplateError::TemplateNotFound(template_name.to_string()))?;

        Ok(self.render_template(&template.content, data))
    }

    fn render_template(&self, template: &str, data: &TemplateData) -> String {
        let mut result = template.to_string();

        // 替换变量
        for (key, value) in &data.variables {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }

        // 处理条件
        result = self.process_conditionals(&result, data);

        // 处理循环
        result = self.process_loops(&result, data);

        result
    }

    fn process_conditionals(&self, template: &str, data: &TemplateData) -> String {
        // 简化的条件处理
        let mut result = template.to_string();

        // 处理 {{#if key}}...{{/if}}
        while let Some(start) = result.find("{{#if ") {
            if let Some(end_tag) = result[start..].find("{{/if}}") {
                let key_start = start + 6;
                let key_end = result[key_start..].find('}').unwrap_or(0) + key_start;
                let key = &result[key_start..key_end].trim();

                let condition = data.variables.get(key).map(|v| v != "false" && v != "").unwrap_or(false);

                let content_start = result[start..].find("}}").unwrap_or(0) + start + 2;
                let content_end = start + end_tag;
                let content = if condition {
                    result[content_start..content_end].to_string()
                } else {
                    String::new()
                };

                let full_end = start + end_tag + 7; // {{/if}} 长度
                result = format!("{}{}{}", &result[..start], content, &result[full_end..]);
            } else {
                break;
            }
        }

        result
    }

    fn process_loops(&self, template: &str, data: &TemplateData) -> String {
        // 简化的循环处理
        let mut result = template.to_string();

        // 处理 {{#each items}}...{{/each}}
        while let Some(start) = result.find("{{#each ") {
            if let Some(end_tag) = result[start..].find("{{/each}}") {
                let key_start = start + 8;
                let key_end = result[key_start..].find('}').unwrap_or(0) + key_start;
                let key = &result[key_start..key_end].trim();

                let items = data.lists.get(key).cloned().unwrap_or_default();
                let content_start = result[start..].find("}}").unwrap_or(0) + start + 2;
                let content_end = start + end_tag;
                let template_content = &result[content_start..content_end];

                let mut expanded = String::new();
                for item in items {
                    let mut item_content = template_content.to_string();
                    item_content = item_content.replace("{{this}}", &item);
                    expanded.push_str(&item_content);
                }

                let full_end = start + end_tag + 11; // {{/each}} 长度
                result = format!("{}{}{}", &result[..start], expanded, &result[full_end..]);
            } else {
                break;
            }
        }

        result
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemplateData {
    pub variables: std::collections::HashMap<String, String>,
    pub lists: std::collections::HashMap<String, Vec<String>>,
}

impl TemplateData {
    pub fn new() -> Self {
        Self {
            variables: std::collections::HashMap::new(),
            lists: std::collections::HashMap::new(),
        }
    }

    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.variables.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_list(mut self, key: &str, values: Vec<String>) -> Self {
        self.lists.insert(key.to_string(), values);
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Render error: {0}")]
    RenderError(String),
}

/// 内置模板
pub mod builtins {
    use super::*;

    /// API 文档 Markdown 模板
    pub const API_DOC_MARKDOWN: &str = r#"# {{title}}

{{#if description}}
{{description}}
{{/if}}

## 目录

{{#each modules}}
- {{this}}
{{/each}}

## API 详情

{{content}}

---
*Generated by Bee Doc Generator*
"#;

    /// README 模板
    pub const README: &str = r#"# {{project_name}}

{{#if description}}
{{description}}
{{/if}}

## 安装

```bash
cargo add {{project_name}}
```

## 使用

```rust
use {{project_name}};
```

## 许可证

{{license}}
"#;

    /// CHANGELOG 模板
    pub const CHANGELOG: &str = r#"# Changelog

{{#each versions}}
## {{this}}

### Changed

### Added

### Fixed

{{/each}}
"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render() {
        let mut engine = TemplateEngine::new();
        engine.register_template("test".to_string(), "Hello {{name}}!".to_string());

        let data = TemplateData::new().with("name", "World");
        let result = engine.render("test", &data).unwrap();

        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_conditional_render() {
        let mut engine = TemplateEngine::new();
        engine.register_template(
            "conditional".to_string(),
            "{{#if show}}visible{{/if}}{{#if hide}}hidden{{/if}}".to_string()
        );

        let data = TemplateData::new()
            .with("show", "true")
            .with("hide", "false");
        let result = engine.render("conditional", &data).unwrap();

        assert_eq!(result, "visible");
    }
}
