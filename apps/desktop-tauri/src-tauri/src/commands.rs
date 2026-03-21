use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    pub api_base_url: String,
    pub turn_uris: Vec<String>,
    pub turn_username_mode: String,
    pub turn_secret: String,
    pub tls_pin_mode: String,
}

#[derive(Default)]
pub struct AppConfigState(pub Mutex<RuntimeConfig>);

#[tauri::command]
pub fn app_health() -> &'static str {
    "ok"
}

#[tauri::command]
pub fn set_runtime_config(state: State<AppConfigState>, cfg: RuntimeConfig) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|_| "config lock poisoned".to_string())?;
    *guard = cfg;
    Ok(())
}

#[tauri::command]
pub fn get_runtime_config(state: State<AppConfigState>) -> Result<RuntimeConfig, String> {
    let guard = state.0.lock().map_err(|_| "config lock poisoned".to_string())?;
    Ok(guard.clone())
}
