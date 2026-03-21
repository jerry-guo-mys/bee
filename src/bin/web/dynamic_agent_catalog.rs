use bee::tools::{tool_call_schema_json, DynamicAgent};

use super::AppState;

pub fn load_dynamic_agents(workspace: &std::path::Path) -> Vec<DynamicAgent> {
    let path = workspace.join("agents.json");
    let data = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub async fn reload_dynamic_agents_into_state(state: &AppState) {
    let dynamic = load_dynamic_agents(&state.workspace);
    if dynamic.is_empty() {
        return;
    }
    let all_tool_list = state
        .tool_descriptions
        .iter()
        .map(|(name, description)| format!("- {}: {}", name, description))
        .collect::<Vec<_>>()
        .join("\n");
    let tool_schema = tool_call_schema_json();
    let mut prompts = state.assistant_prompts.write().await;
    let mut skills = state.assistant_skills.write().await;
    for agent in &dynamic {
        if !prompts.contains_key(&agent.id) {
            let prompt = build_dynamic_agent_prompt(agent, &all_tool_list, &tool_schema);
            prompts.insert(agent.id.clone(), prompt);
        }
        if !skills.contains_key(&agent.id) {
            skills.insert(
                agent.id.clone(),
                state
                    .tool_descriptions
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect(),
            );
        }
    }
}

pub fn build_dynamic_agent_prompt(
    agent: &DynamicAgent,
    tool_list: &str,
    tool_schema: &str,
) -> String {
    let guidance = agent
        .guidance
        .as_deref()
        .unwrap_or("Follow your role and assist the user.");
    format!(
        "You are a sub-agent with role: {}. Guidance: {}\n\nAvailable tools:\n{}",
        agent.role,
        guidance,
        if tool_list.is_empty() {
            "".to_string()
        } else {
            format!(
                "{}\n\n## Tool call JSON Schema (you must output valid JSON matching this)\n```json\n{}\n```",
                tool_list, tool_schema
            )
        }
    )
}
