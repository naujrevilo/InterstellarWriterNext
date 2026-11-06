pub mod commands;
pub mod domain;
pub mod services;

pub fn run() {
    tauri::Builder::default()
        .manage(commands::workspace::WorkspaceState::default())
        .manage(commands::secrets::SecretsState::new())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace::detect_workspace_kind,
            commands::workspace::open_workspace,
            commands::documents::read_document,
            commands::documents::create_document,
            commands::documents::write_document,
            commands::documents::list_document_backups,
            commands::documents::restore_document_backup,
            commands::documents::move_document_to_trash,
            commands::documents::parse_document,
            commands::documents::serialize_document,
            commands::documents::resolve_image,
            commands::secrets::save_github_token,
            commands::secrets::clear_github_token,
            commands::secrets::github_token_configured,
            commands::git::sync_to_github,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
