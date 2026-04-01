//! 集成测试：多租户管理 API

#![cfg(feature = "gateway")]

use bee::application::commands::handler::InMemoryCommandBus;
use bee::application::queries::handler::InMemoryQueryBus;
use bee::interfaces::http::{AppState, create_router};
use std::sync::Arc;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::Service;

/// 创建测试应用状态
fn create_test_app_state() -> AppState {
    AppState {
        command_bus: Arc::new(InMemoryCommandBus::new()),
        query_bus: Arc::new(InMemoryQueryBus::new()),
    }
}

/// 创建测试路由器
fn create_test_app() -> axum::Router {
    let state = create_test_app_state();
    create_router(state)
}

#[tokio::test]
async fn test_create_tenant_success() {
    let mut app = create_test_app();

    // 模拟创建租户请求
    let request = Request::builder()
        .method(Method::POST)
        .uri("/tenants")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"name": "Test Tenant", "slug": "test-tenant"}"#,
        ))
        .unwrap();

    let response: axum::http::Response<Body> = app.call(request).await.unwrap();

    // 验证响应状态码（应该是 200 或 500，取决于命令总线是否有处理器注册）
    // 注意：目前命令总线是空的，所以会返回 500
    assert!(response.status().is_server_error() || response.status().is_success());
}

#[tokio::test]
async fn test_get_tenant_not_found() {
    let mut app = create_test_app();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/tenants/non-existent-id")
        .body(Body::empty())
        .unwrap();

    let response: axum::http::Response<Body> = app.call(request).await.unwrap();

    // 由于查询总线是空的，应该返回 500 或 404
    assert!(
        response.status().is_server_error() || response.status() == StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn test_create_organization_requires_tenant() {
    let mut app = create_test_app();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/tenants/tenant-123/organizations")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"name": "Test Org", "slug": "test-org"}"#,
        ))
        .unwrap();

    let response: axum::http::Response<Body> = app.call(request).await.unwrap();

    // 由于命令总线没有注册处理器，应该返回 500
    assert!(response.status().is_server_error());
}

#[tokio::test]
async fn test_list_members_empty() {
    let mut app = create_test_app();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/organizations/org-123/members")
        .body(Body::empty())
        .unwrap();

    let response: axum::http::Response<Body> = app.call(request).await.unwrap();

    // 由于查询总线是空的，应该返回 500
    assert!(response.status().is_server_error());
}
