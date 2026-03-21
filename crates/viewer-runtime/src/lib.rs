pub mod clipboard;
pub mod input_capture;
pub mod render;
pub mod webrtc_viewer;

use uuid::Uuid;
use webrtc_viewer::{ConnectedViewerPeer, ViewerPeer, ViewerPeerConfig};

#[derive(Debug, thiserror::Error)]
pub enum ViewerRuntimeError {
    #[error("runtime error: {0}")]
    Runtime(String),
}

#[derive(Debug, Clone)]
pub struct ViewerRuntimeConfig {
    pub account_id: Uuid,
    pub device_id: Uuid,
    pub signaling_url: String,
}

pub struct ViewerRuntime {
    config: ViewerRuntimeConfig,
}

impl ViewerRuntime {
    pub fn new(config: ViewerRuntimeConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> Result<(), ViewerRuntimeError> {
        let _ = &self.config.signaling_url;
        Ok(())
    }

    pub async fn join_session(
        &self,
        peer_config: ViewerPeerConfig,
    ) -> Result<ConnectedViewerPeer, ViewerRuntimeError> {
        ViewerPeer::new(peer_config).connect().await
    }
}
