//! Tauri 命令层：前端唯一入口。明文只在 reveal/copy 时按需返回，不缓存。

mod backup;
mod crypto;
mod db;
mod keychain;

use std::path::PathBuf;

use db::{Db, EntryDto, PlatformDto};
use serde::Serialize;
use tauri::State;

struct AppState {
    db: Db,
}

#[derive(Serialize)]
struct VaultState {
    platforms: Vec<PlatformDto>,
    entries: Vec<EntryDto>,
}

#[tauri::command]
fn list_state(state: State<AppState>) -> Result<VaultState, String> {
    Ok(VaultState {
        platforms: state.db.list_platforms()?,
        entries: state.db.list_entries()?,
    })
}

#[tauri::command]
fn add_platform(
    state: State<AppState>,
    name: String,
    color: Option<String>,
    endpoint_openai: Option<String>,
    endpoint_anthropic: Option<String>,
) -> Result<PlatformDto, String> {
    state.db.add_platform(
        &name,
        color.as_deref().unwrap_or(""),
        endpoint_openai.as_deref().unwrap_or(""),
        endpoint_anthropic.as_deref().unwrap_or(""),
    )
}

#[tauri::command]
fn rename_platform(state: State<AppState>, id: String, name: String) -> Result<(), String> {
    state.db.rename_platform(&id, &name)
}

#[tauri::command]
fn set_platform_endpoints(
    state: State<AppState>,
    id: String,
    openai: String,
    anthropic: String,
) -> Result<(), String> {
    state.db.set_platform_endpoints(&id, &openai, &anthropic)
}

#[tauri::command]
fn delete_platform(state: State<AppState>, id: String) -> Result<(), String> {
    state.db.delete_platform(&id)
}

#[tauri::command]
fn add_entry(
    state: State<AppState>,
    platform_id: String,
    name: String,
    key: String,
) -> Result<EntryDto, String> {
    state.db.add_entry(&platform_id, &name, &key)
}

#[tauri::command]
fn update_entry(
    state: State<AppState>,
    id: String,
    name: String,
    platform_id: String,
    key: Option<String>,
) -> Result<(), String> {
    state.db.update_entry(&id, &name, &platform_id, key.as_deref())
}

#[tauri::command]
fn delete_entry(state: State<AppState>, id: String) -> Result<(), String> {
    state.db.delete_entry(&id)
}

#[tauri::command]
fn reveal_entry(state: State<AppState>, id: String) -> Result<String, String> {
    state.db.reveal_entry(&id)
}

#[tauri::command]
fn copy_entry(state: State<AppState>, id: String) -> Result<String, String> {
    state.db.copy_entry(&id)
}

#[tauri::command]
fn export_backup(state: State<AppState>, passphrase: String, path: String) -> Result<(), String> {
    let content = backup::export_backup(&state.db, &passphrase)?;
    std::fs::write(&path, content).map_err(|e| format!("写入备份文件失败: {e}"))
}

#[tauri::command]
fn import_backup(state: State<AppState>, passphrase: String, path: String) -> Result<usize, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("读取备份文件失败: {e}"))?;
    backup::import_backup(&state.db, &passphrase, &content)
}

fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    Ok(dir.join("vault.db"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            use tauri::Manager;
            let path = db_path(app.handle())?;
            let master = keychain::get_or_create_master_key()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let db = Db::open(&path, master)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            app.manage(AppState { db });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_state,
            add_platform,
            rename_platform,
            set_platform_endpoints,
            delete_platform,
            add_entry,
            update_entry,
            delete_entry,
            reveal_entry,
            copy_entry,
            export_backup,
            import_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
