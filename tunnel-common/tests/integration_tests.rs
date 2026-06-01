// Comprehensive Integration Tests for ZeroMQ Tunnel Common Types
// Tests cover: message serialization, types, and protocol validation

use anyhow::Result;
use rmp_serde::{from_slice, to_vec_named};

/// Test RegistrationRequest serialization/deserialization (remote mode)
#[test]
fn test_registration_request_remote() -> Result<()> {
    use tunnel_common::*;
    let req = RegistrationRequest::remote("web-443", "127.0.0.1", 443);

    // Serialize
    let bytes = to_vec_named::<RegistrationRequest>(&req)?;
    assert!(!bytes.is_empty());

    // Deserialize
    let deserialized: RegistrationRequest = from_slice(&bytes)?;
    assert_eq!(deserialized.service_id, "web-443");
    assert_eq!(deserialized.forward_mode, ForwardMode::Remote);
    assert_eq!(deserialized.local_port, None);
    assert_eq!(deserialized.target_host, Some("127.0.0.1".to_string()));
    assert_eq!(deserialized.target_port, Some(443));

    Ok(())
}

/// Test RegistrationRequest serialization for local mode
#[test]
fn test_registration_request_local() -> Result<()> {
    use tunnel_common::*;
    let req = RegistrationRequest::local("internal-api", 8080);

    // Serialize
    let bytes = to_vec_named::<RegistrationRequest>(&req)?;
    assert!(!bytes.is_empty());

    // Deserialize
    let deserialized: RegistrationRequest = from_slice(&bytes)?;
    assert_eq!(deserialized.service_id, "internal-api");
    assert_eq!(deserialized.forward_mode, ForwardMode::Local);
    assert_eq!(deserialized.local_port, Some(8080));

    Ok(())
}

/// Test RegistrationRequest from legacy format (remote)
#[test]
fn test_registration_request_from_legacy_remote() -> Result<()> {
    use tunnel_common::*;
    let req = RegistrationRequest::from_legacy("web-443", true, 443);

    assert_eq!(req.service_id, "web-443");
    assert_eq!(req.forward_mode, ForwardMode::Remote);
    assert_eq!(req.target_host, Some("localhost".to_string()));
    assert_eq!(req.target_port, Some(443));

    Ok(())
}

/// Test RegistrationRequest from legacy format (local)
#[test]
fn test_registration_request_from_legacy_local() -> Result<()> {
    use tunnel_common::*;
    let req = RegistrationRequest::from_legacy("internal-api", false, 8080);

    assert_eq!(req.service_id, "internal-api");
    assert_eq!(req.forward_mode, ForwardMode::Local);
    assert_eq!(req.local_port, Some(8080));

    Ok(())
}

/// Test RegistrationResponse serialization/deserialization (success case)
#[test]
fn test_registration_response_success() -> Result<()> {
    use tunnel_common::*;
    let resp = RegistrationResponse {
        success: true,
        service_id: "web-443".to_string(),
        error: None,
    };

    let bytes = to_vec_named::<RegistrationResponse>(&resp)?;
    let deserialized: RegistrationResponse = from_slice(&bytes)?;

    assert!(deserialized.success);
    assert_eq!(deserialized.service_id, "web-443");
    assert!(deserialized.error.is_none());

    Ok(())
}

/// Test RegistrationResponse with error
#[test]
fn test_registration_response_error() -> Result<()> {
    use tunnel_common::*;
    let resp = RegistrationResponse {
        success: false,
        service_id: "web-443".to_string(),
        error: Some("Already registered".to_string()),
    };

    let bytes = to_vec_named::<RegistrationResponse>(&resp)?;
    let deserialized: RegistrationResponse = from_slice(&bytes)?;

    assert!(!deserialized.success);
    assert_eq!(deserialized.error, Some("Already registered".to_string()));

    Ok(())
}

/// Test StreamId creation and conversion
#[test]
fn test_stream_id_creation() {
    use tunnel_common::*;

    let id1 = StreamId::new();
    let id2 = StreamId::new();

    assert_ne!(id1, id2); // Different UUIDs
    assert!(id1.0.to_string().len() == 36); // UUID string format
}

/// Test StreamId from Uuid conversion
#[test]
fn test_stream_id_from_uuid() {
    use tunnel_common::*;
    use uuid::Uuid;
    let uuid = Uuid::new_v4();
    let stream_id: StreamId = uuid.into();

    assert_eq!(stream_id.0, uuid);
}

/// Test ForwardMode from_str conversion (remote)
#[test]
fn test_forward_mode_from_str_remote() {
    use tunnel_common::*;
    let mode = ForwardMode::from_str("remote");
    assert_eq!(mode, Some(ForwardMode::Remote));

    let mode = ForwardMode::from_str("REMOTE");
    assert_eq!(mode, Some(ForwardMode::Remote));
}

/// Test ForwardMode from_str conversion (local)
#[test]
fn test_forward_mode_from_str_local() {
    use tunnel_common::*;
    let mode = ForwardMode::from_str("local");
    assert_eq!(mode, Some(ForwardMode::Local));

    let mode = ForwardMode::from_str("LOCAL");
    assert_eq!(mode, Some(ForwardMode::Local));
}

/// Test ForwardMode from_str with invalid input
#[test]
fn test_forward_mode_from_str_invalid() {
    use tunnel_common::*;
    let mode = ForwardMode::from_str("invalid");
    assert_eq!(mode, None);
}

/// Test AgentIdentity creation and string conversion
#[test]
fn test_agent_identity_creation() {
    use tunnel_common::*;
    let identity = AgentIdentity::new("curve25519:BAAAAAA=".to_string());
    assert_eq!(identity.as_str(), "curve25519:BAAAAAA=");
}

/// Test StreamStartRequest serialization
#[test]
fn test_stream_start_request_serialization() -> Result<()> {
    use tunnel_common::*;
    let req = StreamStartRequest {
        stream_id: StreamId::new(),
        remote_service_id: Some("web-443".to_string()),
    };

    let bytes = to_vec_named::<StreamStartRequest>(&req)?;
    let deserialized: StreamStartRequest = from_slice(&bytes)?;

    assert!(deserialized.stream_id.0.to_string().len() > 0);
    assert_eq!(deserialized.remote_service_id, Some("web-443".to_string()));

    Ok(())
}

/// Test StreamCloseRequest serialization
#[test]
fn test_stream_close_request_serialization() -> Result<()> {
    use tunnel_common::*;
    let req = StreamCloseRequest {
        stream_id: StreamId::new(),
        reason: Some("client closed connection".to_string()),
    };

    let bytes = to_vec_named::<StreamCloseRequest>(&req)?;
    let deserialized: StreamCloseRequest = from_slice(&bytes)?;

    assert!(deserialized.stream_id.0.to_string().len() > 0);
    assert_eq!(
        deserialized.reason,
        Some("client closed connection".to_string())
    );

    Ok(())
}

/// Test Heartbeat message (empty struct serialization)
#[test]
fn test_heartbeat_serialization() -> Result<()> {
    use tunnel_common::*;
    let hb = Heartbeat;
    let bytes = to_vec_named::<Heartbeat>(&hb)?;
    let _deserialized: Heartbeat = from_slice(&bytes).expect("Failed to deserialize heartbeat");

    // Heartbeat is unit struct, just verify it serializes/deserializes
    assert!(true);

    Ok(())
}

/// Test TunnelError variants (just verifying they exist)
#[test]
fn test_tunnel_error_variants() {
    use rmp_serde::encode::Error;
    use tunnel_common::TunnelError;

    // Verify all error variants can be created
    let _err = TunnelError::Serialize(Error::Syntax("test serialization error".to_string()));
    let _deser_err = TunnelError::Deserialize(rmp_serde::decode::Error::Syntax("test deserialization error".to_string()));
    let _err = TunnelError::Zmq("Test ZMQ error".to_string());
    let _err = TunnelError::Tunnel("Unknown tunnel error".to_string());
    let _err = TunnelError::Io(std::io::Error::new(std::io::ErrorKind::Other, "IO error"));

    // Verify all variants have Debug impl
    let _debug: String = format!("{:?}", _err);
    assert!(_debug.len() > 0);
}

/// Test SessionState enum serialization
#[test]
fn test_session_state_serialization() {
    use tunnel_common::SessionState;

    let states: Vec<(&str, SessionState)> = vec![
        ("Active", SessionState::Active),
        ("Closing", SessionState::Closing),
        ("Closed", SessionState::Closed),
    ];

    for (name, state) in states {
        assert_eq!(format!("{:?}", state), name);
    }
}

/// Test message size validation (regression test)
#[test]
fn test_message_size_bounds() -> Result<()> {
    use tunnel_common::*;
    let req = RegistrationRequest::remote("web-443", "127.0.0.1:8080", 8080);
    let bytes = to_vec_named::<RegistrationRequest>(&req)?;

    // Message should be reasonable size (< 1KB)
    assert!(bytes.len() < 1024, "Message too large: {} bytes", bytes.len());
    assert!(bytes.len() > 0, "Message is empty");

    Ok(())
}

/// Test that control messages have consistent format
#[test]
fn test_control_message_format() -> Result<()> {
    use tunnel_common::*;
    // Test a typical registration request
    let req = RegistrationRequest::remote("test-service", "127.0.0.1", 9000);
    let bytes = to_vec_named::<RegistrationRequest>(&req)?;

    // Verify we can deserialize back
    let deserialized: RegistrationRequest = from_slice(&bytes)?;
    assert_eq!(deserialized.service_id, "test-service");
    assert_eq!(deserialized.target_port, Some(9000));

    Ok(())
}

/// Test local vs remote forwarding modes
#[test]
fn test_local_vs_remote_forwarding_modes() {
    use tunnel_common::*;
    // Remote: connects to external service
    let remote = RegistrationRequest::remote("my-api", "api.example.com", 443);
    assert_eq!(remote.forward_mode, ForwardMode::Remote);
    assert!(remote.target_port.is_some());

    // Local: listens on local port and forwards through tunnel
    let local = RegistrationRequest::local("internal-db", 5432);
    assert_eq!(local.forward_mode, ForwardMode::Local);
    assert!(local.local_port.is_some());
}
