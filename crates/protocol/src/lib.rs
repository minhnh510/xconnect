pub mod signal;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    Windows,
    Macos,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceInfo {
    pub device_id: Uuid,
    pub account_id: Uuid,
    pub device_name: String,
    pub platform: DevicePlatform,
    pub trusted: bool,
    pub unattended_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Requested,
    Accepted,
    Connected,
    Ending,
    Ended,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRequest {
    pub session_id: Uuid,
    pub caller_device_id: Uuid,
    pub target_device_id: Uuid,
    pub unattended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDescriptor {
    pub session_id: Uuid,
    pub account_id: Uuid,
    pub caller_device_id: Uuid,
    pub target_device_id: Uuid,
    pub unattended: bool,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputEvent {
    MouseMove {
        x: f32,
        y: f32,
        normalized: bool,
    },
    MouseButton {
        button: u8,
        pressed: bool,
    },
    Wheel {
        delta_x: i32,
        delta_y: i32,
    },
    Key {
        key_code: u32,
        pressed: bool,
        modifiers: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClipboardEvent {
    pub text_utf8: String,
    pub ts_unix_ms: i64,
}
