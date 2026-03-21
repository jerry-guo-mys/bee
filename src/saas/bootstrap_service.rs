//! 组织初始化应用服务
//!
//! 当前阶段提供一键初始化编排与最小持久化落库能力。

use crate::saas::models::{
    AgentInstance, AgentInstanceStatus, AgentTemplate, Organization, Team, Tenant, Workspace,
};
use crate::saas::sqlite::SaasSqliteStore;
use crate::saas::sqlite_seed_repository::SaasSeedRepository;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationBootstrapRequest {
    pub tenant_id: String,
    pub organization_id: String,
    pub organization_name: String,
    pub industry: IndustryTemplate,
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationBootstrapPlan {
    pub tenant: Tenant,
    pub organization: Organization,
    pub workspace: Workspace,
    pub teams: Vec<Team>,
    pub agent_templates: Vec<AgentTemplate>,
    pub agent_instances: Vec<AgentInstance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationBootstrapResult {
    pub tenant_id: String,
    pub organization_id: String,
    pub workspace_id: String,
    pub team_count: usize,
    pub agent_template_count: usize,
    pub agent_instance_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndustryTemplate {
    General,
    SalesService,
    MarketingStudio,
    RecruitingAgency,
    SoftwareDelivery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTemplate {
    pub code: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub default_agents: &'static [AgentTemplateSeed],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTemplateSeed {
    pub code: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub fn build_bootstrap_plan(req: &OrganizationBootstrapRequest) -> OrganizationBootstrapPlan {
    let now = chrono::Utc::now().to_rfc3339();
    let team_templates = templates_for_industry(req.industry);

    let tenant = Tenant {
        id: req.tenant_id.clone(),
        name: format!("{} Tenant", req.organization_name),
        status: crate::saas::models::TenantStatus::Active,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let organization = Organization {
        id: req.organization_id.clone(),
        tenant_id: req.tenant_id.clone(),
        name: req.organization_name.clone(),
        slug: Some(slugify(&req.organization_name)),
        industry: Some(industry_code(req.industry).to_string()),
        description: Some(format!(
            "Bootstrapped from {} template",
            industry_label(req.industry)
        )),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let workspace = Workspace {
        id: req.workspace_id.clone(),
        tenant_id: req.tenant_id.clone(),
        organization_id: req.organization_id.clone(),
        team_id: None,
        name: format!("{} Workspace", req.organization_name),
        root_path: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    let mut teams = Vec::new();
    let mut agent_templates = Vec::new();
    let mut agent_instances = Vec::new();
    for team_template in team_templates {
        let team_id = format!("team-{}-{}", req.organization_id, team_template.code);
        teams.push(Team {
            id: team_id.clone(),
            tenant_id: req.tenant_id.clone(),
            organization_id: req.organization_id.clone(),
            name: team_template.name.to_string(),
            code: Some(team_template.code.to_string()),
            description: Some(team_template.description.to_string()),
            parent_team_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        });

        for agent in team_template.default_agents {
            agent_templates.push(AgentTemplate {
                id: format!("template-{}", agent.code),
                tenant_id: req.tenant_id.clone(),
                name: agent.name.to_string(),
                description: Some(agent.description.to_string()),
                prompt: Some(agent.description.to_string()),
                tool_ids: Vec::new(),
                model_id: None,
                knowledge_base_ids: Vec::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
            });
            agent_instances.push(AgentInstance {
                id: format!("agent-{}-{}", team_template.code, agent.code),
                tenant_id: req.tenant_id.clone(),
                organization_id: req.organization_id.clone(),
                team_id: Some(team_id.clone()),
                template_id: format!("template-{}", agent.code),
                name: agent.name.to_string(),
                status: AgentInstanceStatus::Active,
                prompt_override: Some(agent.description.to_string()),
                tool_ids_override: Vec::new(),
                model_id_override: None,
                knowledge_base_ids_override: Vec::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        }
    }

    OrganizationBootstrapPlan {
        tenant,
        organization,
        workspace,
        teams,
        agent_templates,
        agent_instances,
    }
}

pub fn persist_bootstrap_plan(
    store: &SaasSqliteStore,
    plan: &OrganizationBootstrapPlan,
) -> anyhow::Result<OrganizationBootstrapResult> {
    let repo = SaasSeedRepository::new(store);
    repo.create_tenant(&plan.tenant)?;
    repo.create_organization(&plan.organization)?;
    repo.create_workspace(&plan.workspace)?;
    for team in &plan.teams {
        repo.create_team(team)?;
    }
    for template in &plan.agent_templates {
        repo.upsert_agent_template(template)?;
    }
    for agent in &plan.agent_instances {
        repo.create_agent_instance(agent)?;
    }

    Ok(OrganizationBootstrapResult {
        tenant_id: plan.tenant.id.clone(),
        organization_id: plan.organization.id.clone(),
        workspace_id: plan.workspace.id.clone(),
        team_count: plan.teams.len(),
        agent_template_count: plan.agent_templates.len(),
        agent_instance_count: plan.agent_instances.len(),
    })
}

pub fn templates_for_industry(industry: IndustryTemplate) -> &'static [TeamTemplate] {
    match industry {
        IndustryTemplate::General => GENERAL_TEMPLATES,
        IndustryTemplate::SalesService => SALES_SERVICE_TEMPLATES,
        IndustryTemplate::MarketingStudio => MARKETING_TEMPLATES,
        IndustryTemplate::RecruitingAgency => RECRUITING_TEMPLATES,
        IndustryTemplate::SoftwareDelivery => SOFTWARE_TEMPLATES,
    }
}

fn slugify(name: &str) -> String {
    let slug = name
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    slug.trim_matches('-').to_string()
}

fn industry_code(industry: IndustryTemplate) -> &'static str {
    match industry {
        IndustryTemplate::General => "general",
        IndustryTemplate::SalesService => "sales_service",
        IndustryTemplate::MarketingStudio => "marketing_studio",
        IndustryTemplate::RecruitingAgency => "recruiting_agency",
        IndustryTemplate::SoftwareDelivery => "software_delivery",
    }
}

fn industry_label(industry: IndustryTemplate) -> &'static str {
    match industry {
        IndustryTemplate::General => "general",
        IndustryTemplate::SalesService => "sales service",
        IndustryTemplate::MarketingStudio => "marketing studio",
        IndustryTemplate::RecruitingAgency => "recruiting agency",
        IndustryTemplate::SoftwareDelivery => "software delivery",
    }
}

const SALES_AGENTS: &[AgentTemplateSeed] = &[
    AgentTemplateSeed {
        code: "lead",
        name: "销售线索助手",
        description: "负责线索收集、线索分级与下一步跟进建议。",
    },
    AgentTemplateSeed {
        code: "crm",
        name: "客户跟进助手",
        description: "负责客户推进、纪要整理与成交提醒。",
    },
];

const SERVICE_AGENTS: &[AgentTemplateSeed] = &[
    AgentTemplateSeed {
        code: "support",
        name: "客服助手",
        description: "负责问题分流、FAQ 回复与工单摘要。",
    },
    AgentTemplateSeed {
        code: "qa",
        name: "服务质检助手",
        description: "负责服务质量抽检与升级提醒。",
    },
];

const MARKETING_AGENTS: &[AgentTemplateSeed] = &[
    AgentTemplateSeed {
        code: "content",
        name: "内容策划助手",
        description: "负责选题、内容排期与文案草稿。",
    },
    AgentTemplateSeed {
        code: "growth",
        name: "增长投放助手",
        description: "负责渠道建议、投放复盘与增长实验设计。",
    },
];

const RECRUITING_AGENTS: &[AgentTemplateSeed] = &[
    AgentTemplateSeed {
        code: "sourcing",
        name: "招聘搜寻助手",
        description: "负责 JD 对齐、候选人筛选与初步推荐。",
    },
    AgentTemplateSeed {
        code: "interview",
        name: "面试流程助手",
        description: "负责面试安排、反馈汇总与候选人推进。",
    },
];

const SOFTWARE_AGENTS: &[AgentTemplateSeed] = &[
    AgentTemplateSeed {
        code: "delivery",
        name: "交付协同助手",
        description: "负责需求拆分、里程碑同步与风险跟踪。",
    },
    AgentTemplateSeed {
        code: "qa",
        name: "研发质检助手",
        description: "负责测试建议、验收清单与发布准备。",
    },
];

const GENERAL_TEMPLATES: &[TeamTemplate] = &[
    TeamTemplate {
        code: "sales",
        name: "销售团队",
        description: "负责线索管理与客户推进。",
        default_agents: SALES_AGENTS,
    },
    TeamTemplate {
        code: "service",
        name: "客服团队",
        description: "负责客户支持与问题响应。",
        default_agents: SERVICE_AGENTS,
    },
    TeamTemplate {
        code: "marketing",
        name: "市场团队",
        description: "负责品牌传播与增长活动。",
        default_agents: MARKETING_AGENTS,
    },
    TeamTemplate {
        code: "hr",
        name: "HR 团队",
        description: "负责招聘与组织支持。",
        default_agents: RECRUITING_AGENTS,
    },
    TeamTemplate {
        code: "engineering",
        name: "研发团队",
        description: "负责需求交付与质量保障。",
        default_agents: SOFTWARE_AGENTS,
    },
];

const SALES_SERVICE_TEMPLATES: &[TeamTemplate] = &[
    TeamTemplate {
        code: "sales",
        name: "销售团队",
        description: "负责销售线索、商机推进与成交分析。",
        default_agents: SALES_AGENTS,
    },
    TeamTemplate {
        code: "service",
        name: "客服团队",
        description: "负责客户支持、工单管理与满意度回访。",
        default_agents: SERVICE_AGENTS,
    },
];

const MARKETING_TEMPLATES: &[TeamTemplate] = &[
    TeamTemplate {
        code: "marketing",
        name: "市场团队",
        description: "负责内容策划、投放增长与活动运营。",
        default_agents: MARKETING_AGENTS,
    },
    TeamTemplate {
        code: "sales",
        name: "商务团队",
        description: "负责商机接入、客户沟通与成交推进。",
        default_agents: SALES_AGENTS,
    },
];

const RECRUITING_TEMPLATES: &[TeamTemplate] = &[TeamTemplate {
    code: "hr",
    name: "招聘团队",
    description: "负责岗位管理、候选人筛选与流程推进。",
    default_agents: RECRUITING_AGENTS,
}];

const SOFTWARE_TEMPLATES: &[TeamTemplate] = &[
    TeamTemplate {
        code: "engineering",
        name: "研发团队",
        description: "负责研发交付、质量保障与发布协同。",
        default_agents: SOFTWARE_AGENTS,
    },
    TeamTemplate {
        code: "service",
        name: "客户成功团队",
        description: "负责上线支持、客户反馈与续约准备。",
        default_agents: SERVICE_AGENTS,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_bootstrap_plan_general() {
        let plan = build_bootstrap_plan(&OrganizationBootstrapRequest {
            tenant_id: "tenant-1".to_string(),
            organization_id: "org-1".to_string(),
            organization_name: "Acme Org".to_string(),
            industry: IndustryTemplate::General,
            workspace_id: "ws-1".to_string(),
        });

        assert_eq!(plan.organization.name, "Acme Org");
        assert_eq!(plan.teams.len(), 5);
        assert!(!plan.agent_templates.is_empty());
        assert!(!plan.agent_instances.is_empty());
    }
}
