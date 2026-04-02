//! 集成测试：WebSocket 网关多租户消息处理

#![cfg(feature = "gateway")]

use bee::gateway::{ClientInfo, GatewayMessage, MemberDto, MessageType, SpokeType};

/// 测试 WebSocket 消息序列化
#[test]
fn test_websocket_message_serialization() {
    let msg = GatewayMessage::new(
        Some("session-123".to_string()),
        MessageType::CreateTenant {
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
        },
    );

    let json = serde_json::to_string(&msg).unwrap();
    println!("JSON output: {}", json);
    assert!(json.contains("create_tenant"));
    assert!(json.contains("test-tenant"));
}

/// 测试 WebSocket 消息反序列化
#[test]
fn test_websocket_message_deserialization() {
    let json = r#"{
        "id": "msg-123",
        "session_id": "session-456",
        "timestamp": 1234567890,
        "message": {
            "type": "create_tenant",
            "name": "Test Tenant",
            "slug": "test-tenant"
        }
    }"#;

    let msg: GatewayMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.session_id, Some("session-456".to_string()));

    if let MessageType::CreateTenant { name, slug } = msg.message {
        assert_eq!(name, "Test Tenant");
        assert_eq!(slug, "test-tenant");
    } else {
        panic!("Expected CreateTenant message type");
    }
}

/// 测试成员列表消息序列化
#[test]
fn test_members_list_serialization() {
    let members = vec![MemberDto {
        id: "member-1".to_string(),
        user_id: "user-1".to_string(),
        display_name: Some("Test User".to_string()),
        email: Some("test@example.com".to_string()),
        role: "Member".to_string(),
        status: "active".to_string(),
        team_name: None,
        joined_at: "2026-03-31T00:00:00Z".to_string(),
    }];

    let msg = GatewayMessage::new(
        Some("session-123".to_string()),
        MessageType::MembersList { members },
    );

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("members_list"));
    assert!(json.contains("test@example.com"));
}

/// 测试认证消息序列化
#[test]
fn test_auth_message_serialization() {
    let client_info = ClientInfo {
        client_id: "client-123".to_string(),
        platform: SpokeType::Web,
        display_name: Some("Test User".to_string()),
        metadata: None,
    };

    let msg = GatewayMessage::new(
        None,
        MessageType::Auth {
            token: Some("jwt-token".to_string()),
            client_info,
        },
    );

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("auth"));
    assert!(json.contains("jwt-token"));
}

/// 测试操作结果消息序列化
#[test]
fn test_operation_result_serialization() {
    let msg = GatewayMessage::new(
        Some("session-123".to_string()),
        MessageType::OperationResult {
            success: true,
            message: "Tenant created successfully".to_string(),
            data: Some(serde_json::json!({
                "tenant_id": "tenant-123",
                "name": "Test Tenant"
            })),
        },
    );

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("operation_result"));
    assert!(json.contains("success"));
    assert!(json.contains("tenant_id"));
}

/// 测试错误消息序列化
#[test]
fn test_error_message_serialization() {
    let msg = GatewayMessage::error("TEST_ERROR", "This is a test error message");

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("error"));
    assert!(json.contains("TEST_ERROR"));
    assert!(json.contains("This is a test error message"));
}

/// 测试 Ping/Pong 消息序列化
#[test]
fn test_ping_pong_serialization() {
    let ping = GatewayMessage::new(
        None,
        MessageType::Ping {
            timestamp: 1234567890,
        },
    );
    let pong = GatewayMessage::pong(1234567890);

    let ping_json = serde_json::to_string(&ping).unwrap();
    let pong_json = serde_json::to_string(&pong).unwrap();

    assert!(ping_json.contains("ping"));
    assert!(pong_json.contains("pong"));
    assert!(ping_json.contains("1234567890"));
    assert!(pong_json.contains("1234567890"));
}
