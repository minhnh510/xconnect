use std::sync::{Arc, Mutex, OnceLock};

use arboard::Clipboard;
use xconnect_protocol::ClipboardEvent;

use crate::ViewerRuntimeError;

fn system_clipboard() -> Result<&'static Mutex<Clipboard>, ViewerRuntimeError> {
    static INSTANCE: OnceLock<Result<Mutex<Clipboard>, String>> = OnceLock::new();
    let result = INSTANCE.get_or_init(|| {
        Clipboard::new()
            .map(Mutex::new)
            .map_err(|err| format!("clipboard init failed: {err}"))
    });

    match result {
        Ok(m) => Ok(m),
        Err(err) => Err(ViewerRuntimeError::Runtime(err.clone())),
    }
}

#[derive(Clone, Default)]
pub struct ClipboardSync {
    latest: Arc<Mutex<Option<String>>>,
}

impl ClipboardSync {
    pub fn apply_remote(&self, event: ClipboardEvent) -> Result<(), ViewerRuntimeError> {
        {
            let mut latest = self.latest.lock().map_err(|_| {
                ViewerRuntimeError::Runtime("clipboard state lock poisoned".to_string())
            })?;
            *latest = Some(event.text_utf8.clone());
        }

        let clipboard = system_clipboard()?;
        clipboard
            .lock()
            .map_err(|_| ViewerRuntimeError::Runtime("clipboard lock poisoned".to_string()))?
            .set_text(event.text_utf8)
            .map_err(|err| ViewerRuntimeError::Runtime(format!("set clipboard failed: {err}")))?;

        Ok(())
    }

    pub fn poll_local_text(&self) -> Result<Option<String>, ViewerRuntimeError> {
        let clipboard = system_clipboard()?;
        let text = clipboard
            .lock()
            .map_err(|_| ViewerRuntimeError::Runtime("clipboard lock poisoned".to_string()))?
            .get_text()
            .ok();

        if let Some(value) = text {
            let mut latest = self.latest.lock().map_err(|_| {
                ViewerRuntimeError::Runtime("clipboard state lock poisoned".to_string())
            })?;
            if latest.as_ref() != Some(&value) {
                *latest = Some(value.clone());
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    pub fn latest_text(&self) -> Option<String> {
        self.latest.lock().ok().and_then(|g| g.clone())
    }
}
