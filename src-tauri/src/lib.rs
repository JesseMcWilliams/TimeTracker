mod commands;
mod db;
mod domain;

use std::sync::Mutex;
use tauri::Manager;

use commands::AppState;

/// Returns the target (x, y) — in the same units as `window_size`/`monitor_size` — for
/// a named screen position, or `None` for "default" (leave the OS/window-manager's
/// initial placement alone) or an unrecognized value.
fn launch_position_offset(position: &str, monitor_size: (f64, f64), window_size: (f64, f64)) -> Option<(f64, f64)> {
    let (mw, mh) = monitor_size;
    let (ww, wh) = window_size;
    let (left, center_x, right) = (0.0, (mw - ww) / 2.0, mw - ww);
    let (top, center_y, bottom) = (0.0, (mh - wh) / 2.0, mh - wh);
    match position {
        "top-left" => Some((left, top)),
        "top-center" => Some((center_x, top)),
        "top-right" => Some((right, top)),
        "middle-left" => Some((left, center_y)),
        "center" => Some((center_x, center_y)),
        "middle-right" => Some((right, center_y)),
        "bottom-left" => Some((left, bottom)),
        "bottom-center" => Some((center_x, bottom)),
        "bottom-right" => Some((right, bottom)),
        _ => None,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("timetracker.sqlite");
            let conn = db::open(&db_path)?;

            if let Ok(Some(profile)) = domain::user_profile::get_user_profile(&conn) {
                if let Some(window) = app.get_webview_window("main") {
                    if let (Some(width), Some(height)) = (profile.window_width, profile.window_height) {
                        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                            width: width as f64,
                            height: height as f64,
                        }));
                    }

                    if profile.launch_position == "custom" {
                        if let (Some(x), Some(y)) = (profile.window_x, profile.window_y) {
                            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                                x: x as f64,
                                y: y as f64,
                            }));
                        }
                    } else if let (Ok(Some(monitor)), Ok(window_size)) = (window.current_monitor(), window.outer_size())
                    {
                        let monitor_size = monitor.size();
                        if let Some((x, y)) = launch_position_offset(
                            &profile.launch_position,
                            (monitor_size.width as f64, monitor_size.height as f64),
                            (window_size.width as f64, window_size.height as f64),
                        ) {
                            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                                x: x as i32,
                                y: y as i32,
                            }));
                        }
                    }
                }
            }

            app.manage(AppState {
                conn: Mutex::new(conn),
                db_path,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_client,
            commands::update_client,
            commands::list_clients,
            commands::create_contract,
            commands::list_contracts,
            commands::update_contract_rate,
            commands::update_contract_details,
            commands::archive_contract,
            commands::restore_contract,
            commands::archive_client,
            commands::restore_client,
            commands::start_timer,
            commands::stop_timer,
            commands::get_active_timers,
            commands::create_manual_entry,
            commands::update_entry,
            commands::update_entry_metadata,
            commands::delete_entry,
            commands::restore_entry,
            commands::list_deleted_entries,
            commands::list_entries,
            commands::list_tags,
            commands::add_tag_to_entry,
            commands::remove_tag_from_entry,
            commands::create_tracking_code,
            commands::list_tracking_codes,
            commands::archive_tracking_code,
            commands::restore_tracking_code,
            commands::list_archived_tracking_codes,
            commands::get_user_profile,
            commands::save_user_profile,
            commands::generate_report,
            commands::generate_timesheets,
            commands::get_default_output_folder,
            commands::import_time_entries,
            commands::list_import_templates,
            commands::create_import_template,
            commands::update_import_template,
            commands::backup_all_data,
            commands::restore_from_backups,
            commands::purge_deleted_entries,
            commands::purge_archived_contracts,
            commands::purge_archived_clients,
            commands::purge_archived_categories,
            commands::purge_all_data,
            commands::get_window_geometry,
            commands::get_database_size,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
