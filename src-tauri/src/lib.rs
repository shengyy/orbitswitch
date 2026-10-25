//! OrbitSwitch: switch Antigravity, Codex and Claude accounts from the menu bar or tray.

mod accounts;
mod antigravity;
mod autostart;
mod commands;
mod error;
mod google;
mod model;
mod native;
mod provider;
mod quota;
mod registry;
mod scheduler;
mod secret_store;
mod shell;
mod state;
mod switcher;
#[cfg(test)]
mod system_tests;
mod tokens;
mod tray;
mod updater;
mod vault;

use std::sync::Arc;

use tauri::Manager;

use crate::state::Orbit;

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just reveals the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            shell::show(app)
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .build(),
        )
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let orbit = Arc::new(Orbit::new(app.handle().clone())?);
            app.manage(orbit.clone());
            app.manage(shell::ShellState::default());
            shell::create(app.handle())?;
            tray::create(app.handle())?;
            scheduler::start(orbit);
            Ok(())
        })
        .on_window_event(shell::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::refresh,
            commands::add_account,
            commands::cancel_add_account,
            commands::import_account,
            commands::remove_account,
            commands::switch_account,
            commands::running_processes,
            commands::stop_client,
            commands::restart_client,
            commands::resize_panel,
            commands::reveal_logs,
            commands::autostart_enabled,
            commands::set_autostart,
            commands::check_update,
            commands::install_update,
            commands::quit,
        ])
        .run(tauri::generate_context!())
        .expect("OrbitSwitch failed to start");
}
