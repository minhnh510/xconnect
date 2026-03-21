pub mod capture_macos;
pub mod capture_windows;
pub mod clipboard;
pub mod encoder_h264;
pub mod input_apply;
pub mod webrtc_host;

use std::time::Duration;

use encoder_h264::{EncoderConfig, H264Encoder};
use uuid::Uuid;
use webrtc_host::{HostPeer, HostPeerConfig};

#[derive(Debug, thiserror::Error)]
pub enum HostRuntimeError {
    #[error("unsupported platform path: {0}")]
    Unsupported(&'static str),
    #[error("runtime error: {0}")]
    Runtime(String),
}

#[derive(Debug, Clone)]
pub struct HostRuntimeConfig {
    pub account_id: Uuid,
    pub device_id: Uuid,
    pub signaling_url: String,
}

#[derive(Debug, Clone)]
pub struct RawFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct HostRuntime {
    config: HostRuntimeConfig,
}

impl HostRuntime {
    pub fn new(config: HostRuntimeConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> Result<(), HostRuntimeError> {
        let _ = &self.config.signaling_url;
        Ok(())
    }

    pub async fn stream_offer_session(
        &self,
        peer_config: HostPeerConfig,
        max_frames: Option<usize>,
    ) -> Result<(), HostRuntimeError> {
        let peer = HostPeer::new(peer_config).connect().await?;
        let mut emitted = 0usize;

        #[cfg(target_os = "windows")]
        {
            let mut capture = capture_windows::WindowsCapture::start()?;
            let first = capture.next_frame()?;
            let mut encoder = H264Encoder::new(EncoderConfig {
                width: first.width,
                height: first.height,
                fps: 60,
                bitrate_kbps: 6_000,
            })?;
            let encoded = encoder.encode(&first.rgba)?;
            if !encoded.is_empty() {
                peer.publish_h264_frame(encoded, 16).await?;
                emitted += 1;
            }
            if max_frames.is_some_and(|limit| emitted >= limit) {
                return Ok(());
            }
            loop {
                let frame = capture.next_frame()?;
                let encoded = encoder.encode(&frame.rgba)?;
                if encoded.is_empty() {
                    tokio::time::sleep(Duration::from_millis(16)).await;
                    continue;
                }
                peer.publish_h264_frame(encoded, 16).await?;
                emitted += 1;
                if max_frames.is_some_and(|limit| emitted >= limit) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
            return Ok(());
        }

        #[cfg(target_os = "macos")]
        {
            let mut capture = capture_macos::MacosCapture::start()?;
            let first = capture.next_frame()?;
            let mut encoder = H264Encoder::new(EncoderConfig {
                width: first.width,
                height: first.height,
                fps: 60,
                bitrate_kbps: 6_000,
            })?;
            let encoded = encoder.encode(&first.rgba)?;
            if !encoded.is_empty() {
                peer.publish_h264_frame(encoded, 16).await?;
                emitted += 1;
            }
            if max_frames.is_some_and(|limit| emitted >= limit) {
                return Ok(());
            }
            loop {
                let frame = capture.next_frame()?;
                let encoded = encoder.encode(&frame.rgba)?;
                if encoded.is_empty() {
                    tokio::time::sleep(Duration::from_millis(16)).await;
                    continue;
                }
                peer.publish_h264_frame(encoded, 16).await?;
                emitted += 1;
                if max_frames.is_some_and(|limit| emitted >= limit) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
            return Ok(());
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = (peer, emitted, max_frames);
            Err(HostRuntimeError::Unsupported(
                "desktop streaming only supported on windows/macos",
            ))
        }
    }
}
