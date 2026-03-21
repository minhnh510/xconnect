use crate::{HostRuntimeError, RawFrame};

#[cfg(target_os = "windows")]
pub struct WindowsCapture {
    monitor: xcap::Monitor,
}

#[cfg(target_os = "windows")]
impl WindowsCapture {
    pub fn start() -> Result<Self, HostRuntimeError> {
        let monitors = xcap::Monitor::all().map_err(|err| {
            HostRuntimeError::Runtime(format!("enumerate monitors failed: {err}"))
        })?;
        let monitor = monitors
            .into_iter()
            .next()
            .ok_or_else(|| HostRuntimeError::Runtime("no monitor detected".to_string()))?;
        Ok(Self { monitor })
    }

    pub fn next_frame(&mut self) -> Result<RawFrame, HostRuntimeError> {
        let image = self
            .monitor
            .capture_image()
            .map_err(|err| HostRuntimeError::Runtime(format!("capture image failed: {err}")))?;

        Ok(RawFrame {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        })
    }
}

#[cfg(not(target_os = "windows"))]
pub struct WindowsCapture;

#[cfg(not(target_os = "windows"))]
impl WindowsCapture {
    pub fn start() -> Result<Self, HostRuntimeError> {
        Err(HostRuntimeError::Unsupported("windows_capture"))
    }

    pub fn next_frame(&mut self) -> Result<RawFrame, HostRuntimeError> {
        Err(HostRuntimeError::Unsupported("windows_capture"))
    }
}
