use serde_json::Value;
use uuid::Uuid;
use xconnect_protocol::signal::{
    Hello, SessionEnd, SessionResponse, SignalEnvelope, SignalMessage,
};
use xconnect_protocol::{SessionRequest, SessionState};

fn roundtrip(msg: SignalMessage) {
    let envelope = SignalEnvelope::new(Uuid::new_v4(), msg.clone());
    let raw = serde_json::to_string(&envelope).expect("serialize envelope");
    let decoded: SignalEnvelope = serde_json::from_str(&raw).expect("deserialize envelope");

    assert_eq!(decoded.version, envelope.version);
    assert_eq!(decoded.correlation_id, envelope.correlation_id);
    assert_eq!(decoded.message, msg);
}

#[test]
fn serde_roundtrip_hello() {
    roundtrip(SignalMessage::Hello(Hello {
        account_id: Uuid::new_v4(),
        device_id: Uuid::new_v4(),
    }));
}

#[test]
fn serde_roundtrip_session_request() {
    roundtrip(SignalMessage::SessionRequest(SessionRequest {
        session_id: Uuid::new_v4(),
        caller_device_id: Uuid::new_v4(),
        target_device_id: Uuid::new_v4(),
        unattended: true,
    }));
}

#[test]
fn serde_roundtrip_session_state() {
    roundtrip(SignalMessage::SessionState(
        xconnect_protocol::signal::SessionStatePayload {
            session_id: Uuid::new_v4(),
            state: SessionState::Connected,
        },
    ));
}

#[test]
fn serde_roundtrip_response_and_end() {
    roundtrip(SignalMessage::SessionResponse(SessionResponse {
        session_id: Uuid::new_v4(),
        accepted: false,
        reason: Some("device_offline".to_string()),
    }));

    roundtrip(SignalMessage::SessionEnd(SessionEnd {
        session_id: Uuid::new_v4(),
        reason: Some("user_ended".to_string()),
    }));
}

#[test]
fn backward_compat_envelope_shape() {
    let input = r#"{
      "version": 1,
      "correlation_id": "0f34f8c7-1dca-472f-8fa9-2f5cc1e4085f",
      "type": "hello",
      "payload": {
        "account_id": "efc087fb-7267-421c-8c37-8f973eb2de8e",
        "device_id": "d62f2d26-e5ab-4110-84e8-6cbe4ff4a1f2"
      }
    }"#;

    let decoded: SignalEnvelope = serde_json::from_str(input).expect("decode vector");
    let reencoded = serde_json::to_value(decoded).expect("encode value");

    assert_eq!(reencoded["version"], Value::from(1));
    assert_eq!(reencoded["type"], Value::from("hello"));
}
