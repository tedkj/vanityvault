#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod clip;
mod engine;
mod gpu;
mod list;
mod pattern;
mod vault;

use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let path = app.path().app_local_data_dir()?.join("vault.json");
            app.manage::<engine::SharedVault>(Arc::new(Mutex::new(vault::Vault::new(path))));
            // The backup asks before every request it sends (backup::Asking).
            let ask = backup::SharedAsk::default();
            app.manage(ask.clone());
            let wire = backup::Asking::new(backup::Http::from_env(), ask, app.handle().clone());
            app.manage::<backup::SharedBackup>(Mutex::new(backup::Backup::new(Box::new(wire))));
            let engine = engine::Engine::default();
            engine.warm_up(app.handle().clone());
            app.manage(engine);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine::about,
            engine::hunt_estimate,
            engine::hunt_start,
            engine::hunt_stop,
            engine::password_check,
            engine::vault_status,
            engine::vault_create,
            engine::vault_unlock,
            engine::vault_recover,
            engine::vault_lock,
            engine::vault_list,
            engine::vault_check,
            engine::vault_destroy,
            engine::vault_secret,
            engine::vault_export,
            engine::vault_delete,
            backup::backup_status,
            backup::backup_register,
            backup::backup_login,
            backup::backup_recover,
            backup::backup_logout,
            backup::backup_list,
            backup::backup_upload,
            backup::backup_upload_new,
            backup::backup_restore,
            backup::backup_remove,
            backup::backup_password,
            backup::backup_delete_account,
            backup::net_answer,
            clip::copy_private,
        ])
        .run(tauri::generate_context!())
        .expect("VanityVault failed to start");
}
