mod patcher;

#[tauri::command]
async fn patch_game(app: tauri::AppHandle, id: String) -> Result<String, String> {
    // Run blocking task in separate thread
    tauri::async_runtime::spawn_blocking(move || patcher::patch_game(app, id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_game(app: tauri::AppHandle, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || patcher::run_game(app, id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn scan_servers(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    use tauri::Manager;
    let path_resolver = app.path();
    let app_data_dir = path_resolver.app_data_dir().map_err(|e| e.to_string())?;

    let servers_dir = app_data_dir.join("servers");

    if !servers_dir.exists() {
        return Ok(vec![]);
    }

    let mut server_ids = Vec::new();
    let entries = std::fs::read_dir(servers_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    server_ids.push(name_str.to_string());
                }
            }
        }
    }

    Ok(server_ids)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![scan_servers, patch_game, run_game])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
