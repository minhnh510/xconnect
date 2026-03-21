use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{SessionRequest, SessionState, PROTOCOL_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalEnvelope {
    pub version: u16,
    pub correlation_id: Uuid,
    #[serde(flatten)]
    pub message: SignalMessage,
}

impl SignalEnvelope {
    pub fn new(correlation_id: Uuid, message: SignalMessage) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            correlation_id,
            message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum SignalMessage {
    Hello(Hello),
    SessionRequest(SessionRequest),
    SessionResponse(SessionResponse),
    SdpOffer(SdpPayload),
    SdpAnswer(SdpPayload),
    IceCandidate(IceCandidate),
    SessionState(SessionStatePayload),
    SessionEnd(SessionEnd),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hello {
    pub account_id: Uuid,
    pub device_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionResponse {
    pub session_id: Uuid,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdpPayload {
    pub session_id: Uuid,
    pub sdp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IceCandidate {
    pub session_id: Uuid,
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStatePayload {
    pub session_id: Uuid,
    pub state: SessionState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionEnd {
    pub session_id: Uuid,
    pub reason: Option<String>,
}
