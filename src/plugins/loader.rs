//! 插件加载器
//!
//! 提供插件的静态加载和动态加载能力，支持版本检查和签名验证。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::plugins::{Plugin, PluginContext, PluginError, PluginMetadata, PluginState};

/// 插件加载器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginLoaderConfig {
    /// 插件目录路径
    pub plugin_dirs: Vec<PathBuf>,
    /// 是否启用签名验证
    pub enable_signature_verification: bool,
    /// 是否启用版本检查
    pub enable_version_check: bool,
    /// 最低支持的插件 API 版本
    pub min_api_version: String,
    /// 最高支持的插件 API 版本
    pub max_api_version: String,
    /// 允许的插件白名单
    pub allowed_plugins: Option<Vec<String>>,
    /// 禁止的插件黑名单
    pub blocked_plugins: Option<Vec<String>>,
}

impl Default for PluginLoaderConfig {
    fn default() -> Self {
        Self {
            plugin_dirs: vec![PathBuf::from("./plugins")],
            enable_signature_verification: false,
            enable_version_check: true,
            min_api_version: "1.0.0".to_string(),
            max_api_version: "2.0.0".to_string(),
            allowed_plugins: None,
            blocked_plugins: None,
        }
    }
}

impl PluginLoaderConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_plugin_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.plugin_dirs.push(dir.into());
        self
    }

    pub fn with_signature_verification(mut self, enable: bool) -> Self {
        self.enable_signature_verification = enable;
        self
    }

    pub fn with_version_check(mut self, enable: bool) -> Self {
        self.enable_version_check = enable;
        self
    }
}

/// 插件签名信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSignature {
    /// 签名者
    pub signer: String,
    /// 签名时间
    pub signed_at: String,
    /// 签名值
    pub signature: String,
    /// 公钥指纹
    pub key_fingerprint: String,
}

/// 插件清单文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// 插件元数据
    pub metadata: PluginMetadata,
    /// 插件文件列表
    pub files: Vec<String>,
    /// 入口文件
    pub entry_point: String,
    /// 签名信息（可选）
    pub signature: Option<PluginSignature>,
    /// 插件 API 版本
    pub api_version: String,
    /// 配置 schema
    pub config_schema: Option<serde_json::Value>,
}

impl PluginManifest {
    /// 从文件加载清单
    pub fn load_from_file(path: &Path) -> Result<Self, PluginLoaderError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PluginLoaderError::IoError(path.to_path_buf(), e))?;

        let manifest: PluginManifest = serde_json::from_str(&content)
            .map_err(|e| PluginLoaderError::ParseError(e.to_string()))?;

        Ok(manifest)
    }

    /// 验证插件 API 版本
    pub fn validate_api_version(&self, config: &PluginLoaderConfig) -> Result<(), PluginLoaderError> {
        if !config.enable_version_check {
            return Ok(());
        }

        let my_version = &self.api_version;
        let min = &config.min_api_version;
        let max = &config.max_api_version;

        if version_compare(my_version, min).map_or(true, |ord| ord == std::cmp::Ordering::Less) {
            return Err(PluginLoaderError::IncompatibleApiVersion {
                plugin: my_version.clone(),
                required: format!(">= {}", min),
            });
        }

        if version_compare(my_version, max).map_or(true, |ord| ord == std::cmp::Ordering::Greater) {
            return Err(PluginLoaderError::IncompatibleApiVersion {
                plugin: my_version.clone(),
                required: format!("<= {}", max),
            });
        }

        Ok(())
    }

    /// 验证签名
    pub fn verify_signature(&self) -> Result<(), PluginLoaderError> {
        if let Some(signature) = &self.signature {
            // 简化实现：实际应用中应使用加密库验证签名
            if signature.signature.is_empty() {
                return Err(PluginLoaderError::InvalidSignature(
                    "Empty signature".to_string(),
                ));
            }
            tracing::info!("Verified signature for plugin: {}", self.metadata.id);
        }
        Ok(())
    }
}

/// 版本比较辅助函数
fn version_compare(v1: &str, v2: &str) -> Option<std::cmp::Ordering> {
    let parts1: Vec<u32> = v1.split('.').filter_map(|s| s.parse().ok()).collect();
    let parts2: Vec<u32> = v2.split('.').filter_map(|s| s.parse().ok()).collect();

    for (a, b) in parts1.iter().zip(parts2.iter()) {
        match a.cmp(b) {
            std::cmp::Ordering::Equal => continue,
            other => return Some(other),
        }
    }

    Some(parts1.len().cmp(&parts2.len()))
}

/// 插件加载器错误
#[derive(Debug, Error)]
pub enum PluginLoaderError {
    #[error("IO error for path {0}: {1}")]
    IoError(PathBuf, std::io::Error),

    #[error("Failed to parse manifest: {0}")]
    ParseError(String),

    #[error("Incompatible API version: plugin {plugin}, required {required}")]
    IncompatibleApiVersion {
        plugin: String,
        required: String,
    },

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Plugin not in allowed list: {0}")]
    PluginNotAllowed(String),

    #[error("Plugin is blocked: {0}")]
    PluginBlocked(String),

    #[error("Plugin already loaded: {0}")]
    PluginAlreadyLoaded(String),

    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    #[error("Plugin error: {0}")]
    PluginError(#[from] PluginError),
}

/// 已加载的插件信息
pub struct LoadedPlugin {
    /// 插件清单
    pub manifest: PluginManifest,
    /// 插件实例
    pub instance: Arc<tokio::sync::RwLock<Box<dyn Plugin>>>,
    /// 加载时间
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    /// 插件路径
    pub path: PathBuf,
}

impl LoadedPlugin {
    pub fn id(&self) -> &str {
        &self.manifest.metadata.id
    }

    pub fn name(&self) -> &str {
        &self.manifest.metadata.name
    }

    pub fn version(&self) -> &str {
        &self.manifest.metadata.version
    }

    pub async fn state(&self) -> PluginState {
        self.instance.read().await.state()
    }
}

/// 插件加载器
pub struct PluginLoader {
    /// 配置
    config: PluginLoaderConfig,
    /// 已加载的插件
    loaded_plugins: HashMap<String, LoadedPlugin>,
    /// 插件上下文
    context: PluginContext,
}

impl PluginLoader {
    /// 创建新的插件加载器
    pub fn new(config: PluginLoaderConfig, context: PluginContext) -> Self {
        Self {
            config,
            loaded_plugins: HashMap::new(),
            context,
        }
    }

    /// 加载单个插件
    pub async fn load_plugin(&mut self, plugin_path: &Path) -> Result<&LoadedPlugin, PluginLoaderError> {
        // 加载清单
        let manifest_path = plugin_path.join("manifest.json");
        let manifest = PluginManifest::load_from_file(&manifest_path)?;

        // 检查是否在白名单中
        if let Some(allowed) = &self.config.allowed_plugins {
            if !allowed.contains(&manifest.metadata.id) {
                return Err(PluginLoaderError::PluginNotAllowed(manifest.metadata.id.clone()));
            }
        }

        // 检查是否在黑名单中
        if let Some(blocked) = &self.config.blocked_plugins {
            if blocked.contains(&manifest.metadata.id) {
                return Err(PluginLoaderError::PluginBlocked(manifest.metadata.id.clone()));
            }
        }

        // 验证 API 版本
        manifest.validate_api_version(&self.config)?;

        // 验证签名（如果启用）
        if self.config.enable_signature_verification {
            manifest.verify_signature()?;
        }

        // 检查是否已加载
        if self.loaded_plugins.contains_key(&manifest.metadata.id) {
            return Err(PluginLoaderError::PluginAlreadyLoaded(
                manifest.metadata.id.clone(),
            ));
        }

        // 创建插件实例（静态加载方式，实际动态加载需要使用 libloading）
        let plugin_instance = self.create_plugin_instance(&manifest, plugin_path)?;

        // 初始化插件
        let mut plugin_guard = plugin_instance.write().await;
        plugin_guard
            .initialize(&self.context)
            .await
            .map_err(PluginLoaderError::from)?;
        drop(plugin_guard);

        // 记录已加载的插件
        let loaded_plugin = LoadedPlugin {
            manifest,
            instance: plugin_instance,
            loaded_at: chrono::Utc::now(),
            path: plugin_path.to_path_buf(),
        };

        let id = loaded_plugin.id().to_string();
        self.loaded_plugins.insert(id.clone(), loaded_plugin);

        tracing::info!("Loaded plugin: {} v{}", id, self.loaded_plugins[&id].version());

        Ok(&self.loaded_plugins[&id])
    }

    /// 创建插件实例（简化实现）
    fn create_plugin_instance(
        &self,
        _manifest: &PluginManifest,
        _plugin_path: &Path,
    ) -> Result<Arc<tokio::sync::RwLock<Box<dyn Plugin>>>, PluginLoaderError> {
        // 注意：实际的动态加载需要使用 libloading 或类似库
        // 这里提供静态注册的框架
        Err(PluginLoaderError::LoadFailed(
            "Dynamic loading not implemented. Use register_static_plugin instead.".to_string(),
        ))
    }

    /// 注册静态插件（编译时链接）
    pub fn register_static_plugin(
        &mut self,
        plugin: Box<dyn Plugin>,
    ) -> Result<(), PluginLoaderError> {
        let id = plugin.metadata().id.clone();

        if self.loaded_plugins.contains_key(&id) {
            return Err(PluginLoaderError::PluginAlreadyLoaded(id));
        }

        let manifest = PluginManifest {
            metadata: plugin.metadata().clone(),
            files: vec![],
            entry_point: String::new(),
            signature: None,
            api_version: "1.0.0".to_string(),
            config_schema: None,
        };

        let instance = Arc::new(tokio::sync::RwLock::new(plugin));

        let loaded_plugin = LoadedPlugin {
            manifest,
            instance,
            loaded_at: chrono::Utc::now(),
            path: PathBuf::new(),
        };

        self.loaded_plugins.insert(id.clone(), loaded_plugin);
        tracing::info!("Registered static plugin: {}", id);

        Ok(())
    }

    /// 扫描并加载所有插件目录中的插件
    pub async fn scan_and_load_plugins(&mut self) -> Vec<Result<String, PluginLoaderError>> {
        let mut results = Vec::new();

        for plugin_dir in &self.config.plugin_dirs {
            if !plugin_dir.exists() {
                tracing::debug!("Plugin directory does not exist: {:?}", plugin_dir);
                continue;
            }

            match std::fs::read_dir(plugin_dir) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            match self.load_plugin(&path).await {
                                Ok(plugin) => {
                                    results.push(Ok(plugin.id().to_string()));
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to load plugin at {:?}: {}", path, e);
                                    results.push(Err(e));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read plugin directory {:?}: {}", plugin_dir, e);
                }
            }
        }

        results
    }

    /// 获取已加载的插件
    pub fn get_plugin(&self, id: &str) -> Option<&LoadedPlugin> {
        self.loaded_plugins.get(id)
    }

    /// 获取已加载的插件（可变引用）
    pub fn get_plugin_mut(&mut self, id: &str) -> Option<&mut LoadedPlugin> {
        self.loaded_plugins.get_mut(id)
    }

    /// 卸载插件
    pub async fn unload_plugin(&mut self, id: &str) -> Result<(), PluginLoaderError> {
        if let Some(plugin) = self.loaded_plugins.remove(id) {
            let mut plugin_guard = plugin.instance.write().await;
            plugin_guard.shutdown().await.map_err(PluginLoaderError::from)?;
            drop(plugin_guard);
            tracing::info!("Unloaded plugin: {}", id);
        }
        Ok(())
    }

    /// 获取所有已加载的插件 ID
    pub fn loaded_plugin_ids(&self) -> Vec<String> {
        self.loaded_plugins.keys().cloned().collect()
    }

    /// 获取已加载插件数量
    pub fn len(&self) -> usize {
        self.loaded_plugins.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.loaded_plugins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        metadata: PluginMetadata,
    }

    impl TestPlugin {
        fn new(id: &str) -> Self {
            Self {
                metadata: PluginMetadata::new(id, "Test Plugin", "1.0.0"),
            }
        }
    }

    #[async_trait::async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&mut self, _ctx: &PluginContext) -> Result<(), PluginError> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<(), PluginError> {
            Ok(())
        }

        fn state(&self) -> PluginState {
            PluginState::Initialized
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_plugin_loader_config_builder() {
        let config = PluginLoaderConfig::new()
            .with_plugin_dir("./custom_plugins")
            .with_signature_verification(true)
            .with_version_check(false);

        assert_eq!(config.plugin_dirs.len(), 2);
        assert!(config.enable_signature_verification);
        assert!(!config.enable_version_check);
    }

    #[test]
    fn test_version_compare() {
        assert_eq!(
            version_compare("1.0.0", "1.0.0"),
            Some(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            version_compare("1.2.0", "1.0.0"),
            Some(std::cmp::Ordering::Greater)
        );
        assert_eq!(
            version_compare("1.0.0", "1.2.0"),
            Some(std::cmp::Ordering::Less)
        );
        assert_eq!(
            version_compare("1.0.1", "1.0.0"),
            Some(std::cmp::Ordering::Greater)
        );
    }

    #[tokio::test]
    async fn test_register_static_plugin() {
        let config = PluginLoaderConfig::new();
        let context = PluginContext::new("/tmp");
        let mut loader = PluginLoader::new(config, context);

        let plugin = Box::new(TestPlugin::new("test_plugin"));
        loader.register_static_plugin(plugin).unwrap();

        assert_eq!(loader.len(), 1);
        assert!(loader.get_plugin("test_plugin").is_some());
    }

    #[test]
    fn test_plugin_manifest_validation() {
        let manifest = PluginManifest {
            metadata: PluginMetadata::new("test", "Test", "1.0.0"),
            files: vec![],
            entry_point: String::new(),
            signature: None,
            api_version: "1.0.0".to_string(),
            config_schema: None,
        };

        let config = PluginLoaderConfig::new();
        assert!(manifest.validate_api_version(&config).is_ok());

        // 测试版本过低
        let manifest_low = PluginManifest {
            api_version: "0.1.0".to_string(),
            ..manifest.clone()
        };
        assert!(manifest_low.validate_api_version(&config).is_err());

        // 测试版本过高
        let manifest_high = PluginManifest {
            api_version: "3.0.0".to_string(),
            ..manifest
        };
        assert!(manifest_high.validate_api_version(&config).is_err());
    }
}
