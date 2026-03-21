use std::sync::{Arc, Mutex, OnceLock};

use arboard::Clipboard;
use xconnect_protocol::ClipboardEvent;

use crate::HostRuntimeError;

fn system_clipboard() -> Result<&'static Mutex<Clipboard>, HostRuntimeError> {
    static INSTANCE: OnceLock<Result<Mutex<Clipboard>, String>> = OnceLock::new();
    let result = INSTANCE.get_or_init(|| {
        Clipboard::new()
            .map(Mutex::new)
            .map_err(|err| format!("clipboard init failed: {err}"))
    });

    match result {
        Ok(m) => Ok(m),
        Err(err) => Err(HostRuntimeError::Runtime(err.clone())),
    }
}

#[derive(Clone, Default)]
pub struct ClipboardSync {
    latest: Arc<Mutex<Option<String>>>,
}

impl ClipboardSync {
    pub fn set_local(&self, text: String) -> Result<(), HostRuntimeError> {
        {
            let mut latest = self.latest.lock().map_err(|_| {
                HostRuntimeError::Runtime("clipboard state lock poisoned".to_string())
            })?;
            *latest = Some(text.clone());
        }

        let clipboard = system_clipboard()?;
        clipboard
            .lock()
            .map_err(|_| HostRuntimeError::Runtime("clipboard lock poisoned".to_string()))?
            .set_text(text)
            .map_err(|err| HostRuntimeError::Runtime(format!("set clipboard failed: {err}")))?;

        Ok(())
    }

    pub fn poll_local_text(&self) -> Result<Option<String>, HostRuntimeError> {
        let clipboard = system_clipboard()?;
        let text = clipboard
            .lock()
            .map_err(|_| HostRuntimeError::Runtime("clipboard lock poisoned".to_string()))?
            .get_text()
            .ok();

        if let Some(value) = text {
            let mut latest = self.latest.lock().map_err(|_| {
                HostRuntimeError::Runtime("clipboard state lock poisoned".to_string())
            })?;
            if latest.as_ref() != Some(&value) {
                *latest = Some(value.clone());
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    pub fn get_local(&self) -> Option<String> {
        self.latest.lock().ok().and_then(|g| g.clone())
    }

    pub fn apply_remote(&self, event: ClipboardEvent) -> Result<(), HostRuntimeError> {
        self.set_local(event.text_utf8)
    }
}
