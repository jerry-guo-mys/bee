use std::fs;

use bee::tool_policy::{
    classify_query, refine_allowed_tools_for_input, should_use_long_term_memory,
};
use bee::tool_router::deterministic_route;
use bee::tools::{
    ToolCapabilityGroup, ToolCapabilitySubgroup, ToolCostClass, ToolCriticMode, ToolFreshness,
    ToolIntent, ToolMetadata, ToolOutputShape, ToolRisk, ToolScope, ToolUseCase,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenCase {
    name: String,
    input: String,
    expected_query_kind: String,
    expected_direct_tool: Option<String>,
    expected_allowed_tools: Vec<String>,
    forbidden_tools: Vec<String>,
    should_use_memory: bool,
}

fn local_file_tool() -> ToolMetadata {
    ToolMetadata::new(ToolScope::LocalWorkspace, vec![ToolIntent::ReadFile])
        .with_risk(ToolRisk::Low)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_preferred_use_cases(vec![ToolUseCase::LocalWorkspaceInspection])
        .with_disallowed_use_cases(vec![
            ToolUseCase::DirectExplanation,
            ToolUseCase::TimeSensitiveCurrent,
            ToolUseCase::ExternalGitHubRepo,
        ])
        .with_requires_explicit_user_request(true)
        .with_capability(
            ToolCapabilityGroup::LocalWorkspace,
            ToolCapabilitySubgroup::FileRead,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
        )
        .with_preferred_rank(2)
        .with_critic_mode(ToolCriticMode::Skip)
}

fn github_tool() -> ToolMetadata {
    ToolMetadata::new(ToolScope::GitHub, vec![ToolIntent::InspectRepository])
        .with_freshness(ToolFreshness::BestEffort)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_preferred_use_cases(vec![ToolUseCase::ExternalGitHubRepo])
        .with_capability(
            ToolCapabilityGroup::RepositoryAnalysis,
            ToolCapabilitySubgroup::GitHubRepo,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
        )
        .with_preferred_rank(1)
}

fn news_tool() -> ToolMetadata {
    ToolMetadata::new(
        ToolScope::RemoteWeb,
        vec![ToolIntent::FetchWebPage, ToolIntent::Research],
    )
    .with_freshness(ToolFreshness::Live)
    .with_preferred_use_cases(vec![ToolUseCase::News, ToolUseCase::TimeSensitiveCurrent])
    .with_capability(
        ToolCapabilityGroup::RealtimeData,
        ToolCapabilitySubgroup::News,
    )
    .with_costs(
        ToolCostClass::Low,
        ToolCostClass::Low,
        ToolCostClass::Low,
        ToolCostClass::Low,
    )
    .with_preferred_rank(1)
}

fn weather_tool() -> ToolMetadata {
    ToolMetadata::new(
        ToolScope::RemoteWeb,
        vec![ToolIntent::FetchWebPage, ToolIntent::Research],
    )
    .with_freshness(ToolFreshness::Live)
    .with_preferred_use_cases(vec![
        ToolUseCase::Weather,
        ToolUseCase::TimeSensitiveCurrent,
    ])
    .with_capability(
        ToolCapabilityGroup::RealtimeData,
        ToolCapabilitySubgroup::Weather,
    )
    .with_costs(
        ToolCostClass::Low,
        ToolCostClass::Low,
        ToolCostClass::Low,
        ToolCostClass::Low,
    )
    .with_preferred_rank(1)
}

fn search_tool() -> ToolMetadata {
    ToolMetadata::new(ToolScope::RemoteWeb, vec![ToolIntent::FetchWebPage])
        .with_freshness(ToolFreshness::BestEffort)
        .with_output_shape(ToolOutputShape::StructuredJson)
        .with_preferred_use_cases(vec![
            ToolUseCase::DirectExplanation,
            ToolUseCase::TimeSensitiveCurrent,
        ])
        .with_capability(
            ToolCapabilityGroup::WebResearch,
            ToolCapabilitySubgroup::WebFetch,
        )
        .with_costs(
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
            ToolCostClass::Low,
        )
        .with_preferred_rank(3)
}

fn all_tools() -> Vec<(String, ToolMetadata)> {
    vec![
        ("ls".to_string(), local_file_tool()),
        ("cat".to_string(), local_file_tool()),
        ("code_read".to_string(), local_file_tool()),
        (
            "shell".to_string(),
            local_file_tool().with_risk(ToolRisk::High),
        ),
        ("search".to_string(), search_tool()),
        ("github_repo_inspect".to_string(), github_tool()),
        ("news".to_string(), news_tool()),
        ("weather".to_string(), weather_tool()),
    ]
}

#[test]
fn test_tool_routing_golden_dataset() {
    let json = fs::read_to_string("tests/golden/tool_routing_cases.json").unwrap();
    let cases: Vec<GoldenCase> = serde_json::from_str(&json).unwrap();
    let tools = all_tools();

    for case in cases {
        let kind = format!("{:?}", classify_query(&case.input));
        assert_eq!(kind, case.expected_query_kind, "case={}", case.name);

        let decision = refine_allowed_tools_for_input(&case.input, &tools);
        for expected in &case.expected_allowed_tools {
            assert!(
                decision.allowed_tools.iter().any(|tool| tool == expected),
                "case={} missing expected tool {} in {:?}",
                case.name,
                expected,
                decision.allowed_tools
            );
        }
        for forbidden in &case.forbidden_tools {
            assert!(
                !decision.allowed_tools.iter().any(|tool| tool == forbidden),
                "case={} forbidden tool {} present in {:?}",
                case.name,
                forbidden,
                decision.allowed_tools
            );
        }

        let direct_tool = deterministic_route(&case.input, Some(decision.allowed_tools.as_slice()))
            .map(|route| route.tool_name);
        assert_eq!(direct_tool, case.expected_direct_tool, "case={}", case.name);
        assert_eq!(
            should_use_long_term_memory(&case.input),
            case.should_use_memory,
            "case={}",
            case.name
        );
    }
}
