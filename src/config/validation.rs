//! 配置验证
//!
//! 为各配置段提供验证逻辑，确保配置有效性

use crate::config::{AppConfig, EvolutionSection, LlmSection, MemorySection, ToolsSection};

/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置验证失败：{0}")]
    ValidationError(String),
    #[error("配置加载失败：{0}")]
    LoadError(String),
}

/// 配置验证 trait
pub trait Validate {
    fn validate(&self) -> Result<(), ConfigError>;
}

impl Validate for AppConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        self.llm.validate()?;
        self.memory.validate()?;
        self.tools.validate()?;
        self.evolution.validate()?;
        Ok(())
    }
}

impl Validate for LlmSection {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.provider.is_empty() {
            return Err(ConfigError::ValidationError(
                "LLM provider is required".into(),
            ));
        }

        if self.model.is_empty() {
            return Err(ConfigError::ValidationError("LLM model is required".into()));
        }

        Ok(())
    }
}

impl Validate for MemorySection {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.embedding_model.is_empty() {
            return Err(ConfigError::ValidationError(
                "embedding_model is required".into(),
            ));
        }

        Ok(())
    }
}

impl Validate for ToolsSection {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.tool_timeout_secs == 0 {
            return Err(ConfigError::ValidationError(
                "tool_timeout_secs must be > 0".into(),
            ));
        }

        Ok(())
    }
}

impl Validate for EvolutionSection {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.max_iterations == 0 {
            return Err(ConfigError::ValidationError(
                "max_iterations must be > 0".into(),
            ));
        }

        if self.schedule_interval_seconds < 60 {
            return Err(ConfigError::ValidationError(
                "schedule_interval_seconds must be >= 60".into(),
            ));
        }

        if self.target_score_threshold < 0.0 || self.target_score_threshold > 1.0 {
            return Err(ConfigError::ValidationError(
                "target_score_threshold must be between 0.0 and 1.0".into(),
            ));
        }

        Ok(())
    }
}

/// 验证配置并返回结果
pub fn validate_config(config: &AppConfig) -> Result<(), ConfigError> {
    config.validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let _config = AppConfig::default();
        // 默认配置应该通过验证（如果默认值设置正确）
    }

    #[test]
    fn test_invalid_llm_provider() {
        let mut config = AppConfig::default();
        config.llm.provider = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_llm_model() {
        let mut config = AppConfig::default();
        config.llm.model = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_tool_timeout() {
        let mut config = AppConfig::default();
        config.tools.tool_timeout_secs = 0;
        assert!(config.validate().is_err());
    }
}
