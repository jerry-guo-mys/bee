//! SaaS 存储引导
//!
//! 负责初始化 sqlite、执行旧文件数据导入，并返回导入报告。

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::saas::{
    load_platform_agent_templates, LegacyWorkspaceImporter, MigrationReport, SaasSeedRepository,
    SaasSqliteStore,
};

#[derive(Debug, Clone)]
pub struct SaasBootstrapResult {
    pub db_path: PathBuf,
    pub report: MigrationReport,
}

pub fn bootstrap_workspace_saas(workspace: &Path) -> anyhow::Result<SaasBootstrapResult> {
    let runtime_dir = workspace.join(".bee");
    std::fs::create_dir_all(&runtime_dir)
        .with_context(|| format!("create {}", runtime_dir.display()))?;

    let db_path = runtime_dir.join("saas.db");
    let store = SaasSqliteStore::new(&db_path)
        .with_context(|| format!("init saas sqlite {}", db_path.display()))?;
    let importer = LegacyWorkspaceImporter::new(&store);
    let report =
        importer.import_workspace(workspace, "tenant-default", "org-default", "ws-default")?;
    seed_platform_templates(&store, Path::new("config"), "tenant-default")?;

    Ok(SaasBootstrapResult { db_path, report })
}

fn seed_platform_templates(
    store: &SaasSqliteStore,
    config_base: &Path,
    tenant_id: &str,
) -> anyhow::Result<()> {
    let templates = load_platform_agent_templates(config_base, tenant_id)?;
    let repo = SaasSeedRepository::new(store);
    for template in templates {
        repo.upsert_agent_template(&template)?;
    }
    Ok(())
}
