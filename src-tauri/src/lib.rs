pub mod backup;
pub mod commands;
pub mod db;
pub mod error;
pub mod indexing;
pub mod models;
pub mod ollama;
pub mod rag;
pub mod safety;

use db::{open_vault, vault_path, VaultManager, VaultMode, VaultState};
use indexing::worker::IndexingControl;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Second launch attempt: focus the existing window instead of
            // opening a second one against the same SQLite file.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = db::app_data_dir(&handle).expect("resolve app data dir");
            let path = vault_path(&data_dir, VaultMode::Personal);
            let mut state: VaultState = open_vault(&path).expect("open personal vault");
            state.mode = VaultMode::Personal;
            app.manage(VaultManager(Mutex::new(Arc::new(state))));
            app.manage(IndexingControl::default());

            indexing::worker::spawn_worker_loop(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::entries::save_entry,
            commands::entries::get_entry,
            commands::entries::list_entries,
            commands::entries::update_entry,
            commands::entries::delete_entry,
            commands::entries::set_memory_eligibility,
            commands::worries::create_worry,
            commands::worries::update_worry,
            commands::worries::list_worries,
            commands::worries::set_worry_status,
            commands::worries::create_outcome,
            commands::worries::update_outcome,
            commands::worries::delete_outcome,
            commands::worries::create_step,
            commands::worries::set_step_feedback,
            commands::system::get_settings,
            commands::system::grant_local_ai_consent,
            commands::system::withdraw_local_ai_consent,
            commands::system::set_support_country,
            commands::system::check_ollama_runtime,
            commands::system::run_readiness_check_and_select,
            commands::system::pause_indexing,
            commands::system::resume_indexing,
            commands::system::rebuild_index,
            commands::reflect::reflect,
            commands::reflect::save_reflection_as_entry,
            commands::data::export_vault_json,
            commands::data::write_text_export,
            commands::data::preview_vault_import,
            commands::data::import_vault_json,
            commands::data::read_text_import,
            commands::data::export_markdown,
            commands::data::backup_vault,
            commands::data::restore_vault,
            commands::data::erase_vault,
            commands::data::get_vault_mode,
            commands::data::switch_vault_mode,
            commands::data::seed_demo_vault,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Anchor");
}
