//! HTTP 路由定义

use axum::{
    routing::{get, post},
    Router,
};

use super::handlers::*;

/// 创建 REST API 路由
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 租户路由
        .route("/tenants", post(create_tenant))
        .route("/tenants/:tenant_id", get(get_tenant))
        // 组织路由
        .route(
            "/tenants/:tenant_id/organizations",
            post(create_organization),
        )
        .route("/organizations/:organization_id", get(get_organization))
        // 团队路由
        .route(
            "/organizations/:organization_id/teams",
            post(create_team),
        )
        // 成员路由
        .route(
            "/organizations/:organization_id/members",
            post(invite_member).get(list_members),
        )
        .route("/members/:membership_id/accept", post(accept_invite))
        .route("/members/:membership_id/suspend", post(suspend_member))
        .with_state(state)
}
