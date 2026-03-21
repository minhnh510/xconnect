mod commands;

fn main() {
    tauri::Builder::default()
        .manage(commands::AppConfigState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_health,
            commands::set_runtime_config,
            commands::get_runtime_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running xconnect desktop shell");
}
