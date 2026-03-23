//! 文档生成器核心实现

use std::path::Path;
use std::fs;

/// 文档生成器配置
#[derive(Debug, Clone)]
pub struct DocGeneratorConfig {
    pub output_format: OutputFormat,
    pub include_private: bool,
    pub generate_toc: bool,
    pub output_dir: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Html,
    Both,
}

impl Default for DocGeneratorConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Markdown,
            include_private: false,
            generate_toc: true,
            output_dir: "./docs".to_string(),
        }
    }
}

/// 文档生成器
pub struct DocGenerator {
    config: DocGeneratorConfig,
}

/// API 文档结构
#[derive(Debug, Clone)]
pub struct ApiDoc {
    pub title: String,
    pub description: String,
    pub modules: Vec<ModuleDoc>,
    pub types: Vec<TypeDoc>,
    pub functions: Vec<FunctionDoc>,
}

#[derive(Debug, Clone)]
pub struct ModuleDoc {
    pub name: String,
    pub description: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TypeDoc {
    pub name: String,
    pub kind: TypeKind,
    pub description: String,
    pub fields: Vec<FieldDoc>,
    pub methods: Vec<MethodDoc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    TypeAlias,
}

#[derive(Debug, Clone)]
pub struct FieldDoc {
    pub name: String,
    pub type_name: String,
    pub description: String,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Crate,
}

#[derive(Debug, Clone)]
pub struct MethodDoc {
    pub name: String,
    pub signature: String,
    pub description: String,
    pub params: Vec<ParamDoc>,
    pub returns: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParamDoc {
    pub name: String,
    pub type_name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct FunctionDoc {
    pub name: String,
    pub signature: String,
    pub description: String,
    pub params: Vec<ParamDoc>,
    pub returns: Option<String>,
}

impl DocGenerator {
    pub fn new(config: DocGeneratorConfig) -> Self {
        Self { config }
    }

    /// 从 Rust 源码生成文档
    pub fn generate_from_rust(&self, source_path: &Path) -> Result<ApiDoc, DocGeneratorError> {
        let content = fs::read_to_string(source_path)
            .map_err(|e| DocGeneratorError::IoError(source_path.to_path_buf(), e))?;

        self.parse_rust_source(&content)
    }

    /// 解析 Rust 源码
    fn parse_rust_source(&self, content: &str) -> Result<ApiDoc, DocGeneratorError> {
        let mut doc = ApiDoc {
            title: String::new(),
            description: String::new(),
            modules: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut current_doc_comments: Vec<String> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // 收集文档注释
            if line.starts_with("///") || line.starts_with("//!") {
                let comment = line.strip_prefix("///").unwrap_or_else(|| line.strip_prefix("//!").unwrap());
                current_doc_comments.push(comment.trim().to_string());
                i += 1;
                continue;
            }

            // 解析 pub struct
            if line.starts_with("pub struct ") {
                if let Some(struct_doc) = self.parse_struct(line, &current_doc_comments, &lines[i+1..]) {
                    doc.types.push(struct_doc);
                }
                current_doc_comments.clear();
            }
            // 解析 pub enum
            else if line.starts_with("pub enum ") {
                if let Some(enum_doc) = self.parse_enum(line, &current_doc_comments) {
                    doc.types.push(enum_doc);
                }
                current_doc_comments.clear();
            }
            // 解析 pub trait
            else if line.starts_with("pub trait ") {
                if let Some(trait_doc) = self.parse_trait(line, &current_doc_comments) {
                    doc.types.push(trait_doc);
                }
                current_doc_comments.clear();
            }
            // 解析 pub fn
            else if line.starts_with("pub fn ") {
                if let Some(fn_doc) = self.parse_function(line, &current_doc_comments) {
                    doc.functions.push(fn_doc);
                }
                current_doc_comments.clear();
            }
            // 解析 mod
            else if line.starts_with("pub mod ") {
                if let Some(mod_doc) = self.parse_module(line, &current_doc_comments) {
                    doc.modules.push(mod_doc);
                }
                current_doc_comments.clear();
            }
            else {
                current_doc_comments.clear();
            }

            i += 1;
        }

        Ok(doc)
    }

    fn parse_struct(&self, line: &str, comments: &[String], _following_lines: &[&str]) -> Option<TypeDoc> {
        // 提取 struct 名称
        let name = line
            .strip_prefix("pub struct ")?
            .split_whitespace()
            .next()?
            .trim_end_matches('<')
            .trim_end_matches('{')
            .to_string();

        Some(TypeDoc {
            name,
            kind: TypeKind::Struct,
            description: comments.join("\n"),
            fields: Vec::new(), // 简化实现
            methods: Vec::new(),
        })
    }

    fn parse_enum(&self, line: &str, comments: &[String]) -> Option<TypeDoc> {
        let name = line
            .strip_prefix("pub enum ")?
            .split_whitespace()
            .next()?
            .trim_end_matches('<')
            .trim_end_matches('{')
            .to_string();

        Some(TypeDoc {
            name,
            kind: TypeKind::Enum,
            description: comments.join("\n"),
            fields: Vec::new(),
            methods: Vec::new(),
        })
    }

    fn parse_trait(&self, line: &str, comments: &[String]) -> Option<TypeDoc> {
        let name = line
            .strip_prefix("pub trait ")?
            .split_whitespace()
            .next()?
            .trim_end_matches('<')
            .trim_end_matches('{')
            .to_string();

        Some(TypeDoc {
            name,
            kind: TypeKind::Trait,
            description: comments.join("\n"),
            fields: Vec::new(),
            methods: Vec::new(),
        })
    }

    fn parse_function(&self, line: &str, comments: &[String]) -> Option<FunctionDoc> {
        let name = line
            .strip_prefix("pub fn ")?
            .split('(')
            .next()?
            .trim()
            .to_string();

        Some(FunctionDoc {
            name,
            signature: line.to_string(),
            description: comments.join("\n"),
            params: Vec::new(),
            returns: None,
        })
    }

    fn parse_module(&self, line: &str, comments: &[String]) -> Option<ModuleDoc> {
        let name = line
            .strip_prefix("pub mod ")?
            .split_whitespace()
            .next()?
            .trim_end_matches('{')
            .to_string();

        Some(ModuleDoc {
            name,
            description: comments.join("\n"),
            items: Vec::new(),
        })
    }

    /// 生成 Markdown 文档
    pub fn generate_markdown(&self, doc: &ApiDoc) -> String {
        let mut md = String::new();

        // 标题
        md.push_str(&format!("# {}\n\n", doc.title));
        if !doc.description.is_empty() {
            md.push_str(&format!("{}\n\n", doc.description));
        }

        // 目录
        if self.config.generate_toc {
            md.push_str("## 目录\n\n");
            if !doc.modules.is_empty() {
                md.push_str("### 模块\n\n");
                for module in &doc.modules {
                    md.push_str(&format!("- [{}](#module-{})\n", module.name, module.name.to_lowercase()));
                }
                md.push('\n');
            }
            if !doc.types.is_empty() {
                md.push_str("### 类型\n\n");
                for ty in &doc.types {
                    md.push_str(&format!("- [{}](#{}-{})\n", ty.name, format!("{:?}", ty.kind).to_lowercase(), ty.name.to_lowercase()));
                }
                md.push('\n');
            }
            if !doc.functions.is_empty() {
                md.push_str("### 函数\n\n");
                for func in &doc.functions {
                    md.push_str(&format!("- [{}](#function-{})\n", func.name, func.name.to_lowercase()));
                }
                md.push('\n');
            }
        }

        // 模块
        if !doc.modules.is_empty() {
            md.push_str("## 模块\n\n");
            for module in &doc.modules {
                md.push_str(&format!("### {}\n\n", module.name));
                if !module.description.is_empty() {
                    md.push_str(&format!("{}\n\n", module.description));
                }
            }
        }

        // 类型
        if !doc.types.is_empty() {
            md.push_str("## 类型\n\n");
            for ty in &doc.types {
                let kind_str = format!("{:?}", ty.kind);
                md.push_str(&format!("### {} {}\n\n", kind_str, ty.name));
                if !ty.description.is_empty() {
                    md.push_str(&format!("{}\n\n", ty.description));
                }
            }
        }

        // 函数
        if !doc.functions.is_empty() {
            md.push_str("## 函数\n\n");
            for func in &doc.functions {
                md.push_str(&format!("### `{}`\n\n", func.name));
                md.push_str(&format!("```rust\n{}\n```\n\n", func.signature));
                if !func.description.is_empty() {
                    md.push_str(&format!("{}\n\n", func.description));
                }
            }
        }

        md
    }

    /// 生成 HTML 文档
    pub fn generate_html(&self, doc: &ApiDoc) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }}
        h2 {{ color: #555; margin-top: 30px; }}
        h3 {{ color: #666; }}
        code {{ background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }}
        pre {{ background: #f4f4f4; padding: 15px; border-radius: 5px; overflow-x: auto; }}
        .toc {{ background: #f9f9f9; padding: 15px; border-radius: 5px; margin: 20px 0; }}
        .toc ul {{ list-style: none; padding-left: 20px; }}
        .toc a {{ color: #007bff; text-decoration: none; }}
        .toc a:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    {}

    <div class="toc">
        <h2>目录</h2>
        {}
    </div>

    <h2>模块</h2>
    {}

    <h2>类型</h2>
    {}

    <h2>函数</h2>
    {}
</body>
</html>"#,
            doc.title,
            doc.title,
            if doc.description.is_empty() { String::new() } else { format!("<p>{}</p>", doc.description) },
            self.generate_toc_html(doc),
            self.generate_modules_html(doc),
            self.generate_types_html(doc),
            self.generate_functions_html(doc)
        )
    }

    fn generate_toc_html(&self, doc: &ApiDoc) -> String {
        let mut html = String::from("<ul>");
        for module in &doc.modules {
            html.push_str(&format!("<li><a href=\"#module-{}\">{}</a></li>", module.name.to_lowercase(), module.name));
        }
        for ty in &doc.types {
            html.push_str(&format!("<li><a href=\"#{}-{}\">{}</a></li>", format!("{:?}", ty.kind).to_lowercase(), ty.name.to_lowercase(), ty.name));
        }
        for func in &doc.functions {
            html.push_str(&format!("<li><a href=\"#function-{}\">{}</a></li>", func.name.to_lowercase(), func.name));
        }
        html.push_str("</ul>");
        html
    }

    fn generate_modules_html(&self, doc: &ApiDoc) -> String {
        let mut html = String::new();
        for module in &doc.modules {
            html.push_str(&format!("<h3 id=\"module-{}\">{}</h3>", module.name.to_lowercase(), module.name));
            if !module.description.is_empty() {
                html.push_str(&format!("<p>{}</p>", module.description));
            }
        }
        html
    }

    fn generate_types_html(&self, doc: &ApiDoc) -> String {
        let mut html = String::new();
        for ty in &doc.types {
            html.push_str(&format!("<h3 id=\"{}-{}\">{} {}</h3>", format!("{:?}", ty.kind).to_lowercase(), ty.name.to_lowercase(), format!("{:?}", ty.kind), ty.name));
            if !ty.description.is_empty() {
                html.push_str(&format!("<p>{}</p>", ty.description));
            }
        }
        html
    }

    fn generate_functions_html(&self, doc: &ApiDoc) -> String {
        let mut html = String::new();
        for func in &doc.functions {
            html.push_str(&format!("<h3 id=\"function-{}\"><code>{}</code></h3>", func.name.to_lowercase(), func.name));
            html.push_str(&format!("<pre><code>{}</code></pre>", escape_html(&func.signature)));
            if !func.description.is_empty() {
                html.push_str(&format!("<p>{}</p>", escape_html(&func.description)));
            }
        }
        html
    }

    /// 保存文档到文件
    pub fn save_doc(&self, content: &str, filename: &str) -> Result<(), DocGeneratorError> {
        fs::create_dir_all(&self.config.output_dir)
            .map_err(|e| DocGeneratorError::IoError(std::path::PathBuf::from(&self.config.output_dir), e))?;

        let path = Path::new(&self.config.output_dir).join(filename);
        fs::write(&path, content)
            .map_err(|e| DocGeneratorError::IoError(path, e))?;

        Ok(())
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Debug, thiserror::Error)]
pub enum DocGeneratorError {
    #[error("IO error for path {0}: {1}")]
    IoError(std::path::PathBuf, std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rust() {
        let generator = DocGenerator::new(DocGeneratorConfig::default());
        let source = r#"
/// A simple struct
pub struct Foo {
    pub x: i32,
}

/// A simple function
pub fn bar() -> i32 {
    42
}
"#;

        // 简化测试，只验证不 panic
        let doc = generator.parse_rust_source(source).unwrap();
        assert!(!doc.types.is_empty() || !doc.functions.is_empty());
    }
}
