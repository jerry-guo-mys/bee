//! 基础设施层：外部系统适配
//!
//! 提供 LLM 客户端、数据库、文件系统、HTTP 客户端等基础设施实现

pub mod llm;
pub mod memory;
pub mod persistence;
pub mod pool;
pub mod session;
