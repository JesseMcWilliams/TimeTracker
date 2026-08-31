use std::sync::Mutex;
use tauri::{Manager, State};

use crate::domain::backup::{self, BackupFile, PurgeAllResult, RestoreResult};
use crate::domain::contracts::{self, Client, Contract};
use crate::domain::import;
use crate::domain::import_templates::{self, ImportTemplate};
use crate::domain::purge::{self, PurgeResult};
use crate::domain::reports::{self, Report};
use crate::domain::tags::{self, Tag};
use crate::domain::time_entries::{self, EntryFilter, TimeEntry};
use crate::domain::timesheets::{self, TimesheetFile};
use crate::domain::tracking_codes::{self, TrackingCode};
use crate::domain::user_profile::{self, UserProfile};

pub struct AppState {
    pub conn: Mutex<rusqlite::Connection>,
    pub db_path: std::path::PathBuf,
}

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn create_client(state: State<AppState>, name: String, notes: Option<String>) -> CmdResult<i64> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::create_client(&conn, &name, notes.as_deref())
}

#[tauri::command]
pub fn list_clients(state: State<AppState>, include_archived: bool) -> CmdResult<Vec<Client>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::list_clients(&conn, include_archived)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_client(
    state: State<AppState>,
    client_id: i64,
    name: String,
    notes: Option<String>,
    prime: Option<String>,
    sub: Option<String>,
    external_id: Option<String>,
    week_start: String,
    week_end: String,
    default_tracking_code_id: Option<i64>,
    minimum_increment_minutes: Option<i64>,
) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::update_client(
        &conn,
        client_id,
        &name,
        notes.as_deref(),
        prime.as_deref(),
        sub.as_deref(),
        external_id.as_deref(),
        &week_start,
        &week_end,
        default_tracking_code_id,
        minimum_increment_minutes,
    )
}

#[tauri::command]
pub fn archive_client(state: State<AppState>, client_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::archive_client(&conn, client_id)
}

#[tauri::command]
pub fn restore_client(state: State<AppState>, client_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::restore_client(&conn, client_id)
}

#[tauri::command]
pub fn create_contract(
    state: State<AppState>,
    client_id: i64,
    name: String,
    currency: String,
    initial_hourly_rate: f64,
) -> CmdResult<i64> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::create_contract(&conn, client_id, &name, &currency, initial_hourly_rate)
}

#[tauri::command]
pub fn list_contracts(state: State<AppState>, include_archived: bool) -> CmdResult<Vec<Contract>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::list_contracts(&conn, include_archived)
}

#[tauri::command]
pub fn update_contract_rate(state: State<AppState>, contract_id: i64, new_hourly_rate: f64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::update_contract_rate(&conn, contract_id, new_hourly_rate)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_contract_details(
    state: State<AppState>,
    contract_id: i64,
    name: String,
    currency: String,
    external_id: Option<String>,
    start_date: Option<String>,
    notes: Option<String>,
    filename_date: String,
) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::update_contract_details(
        &conn,
        contract_id,
        &name,
        &currency,
        external_id.as_deref(),
        start_date.as_deref(),
        notes.as_deref(),
        &filename_date,
    )
}

#[tauri::command]
pub fn archive_contract(state: State<AppState>, contract_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::archive_contract(&conn, contract_id)
}

#[tauri::command]
pub fn restore_contract(state: State<AppState>, contract_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    contracts::restore_contract(&conn, contract_id)
}

#[tauri::command]
pub fn start_timer(
    state: State<AppState>,
    contract_id: i64,
    notes: Option<String>,
    tracking_code_id: Option<i64>,
) -> CmdResult<i64> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::start_timer(&conn, contract_id, notes.as_deref(), tracking_code_id)
}

#[tauri::command]
pub fn stop_timer(state: State<AppState>, entry_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::stop_timer(&conn, entry_id)
}

#[tauri::command]
pub fn get_active_timers(state: State<AppState>) -> CmdResult<Vec<TimeEntry>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::get_active_timers(&conn)
}

#[tauri::command]
pub fn create_manual_entry(
    state: State<AppState>,
    contract_id: i64,
    started_at: String,
    ended_at: String,
    notes: Option<String>,
    tracking_code_id: Option<i64>,
) -> CmdResult<i64> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::create_manual_entry(
        &conn,
        contract_id,
        &started_at,
        &ended_at,
        notes.as_deref(),
        tracking_code_id,
    )
}

#[tauri::command]
pub fn update_entry(
    state: State<AppState>,
    entry_id: i64,
    started_at: String,
    ended_at: String,
    notes: Option<String>,
    tracking_code_id: Option<i64>,
) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::update_entry(
        &conn,
        entry_id,
        &started_at,
        &ended_at,
        notes.as_deref(),
        tracking_code_id,
    )
}

#[tauri::command]
pub fn update_entry_metadata(
    state: State<AppState>,
    entry_id: i64,
    notes: Option<String>,
    tracking_code_id: Option<i64>,
) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::update_entry_metadata(&conn, entry_id, notes.as_deref(), tracking_code_id)
}

#[tauri::command]
pub fn delete_entry(state: State<AppState>, entry_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::delete_entry(&conn, entry_id)
}

#[tauri::command]
pub fn restore_entry(state: State<AppState>, entry_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::restore_entry(&conn, entry_id)
}

#[tauri::command]
pub fn list_deleted_entries(state: State<AppState>) -> CmdResult<Vec<TimeEntry>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::list_deleted_entries(&conn)
}

#[tauri::command]
pub fn list_entries(state: State<AppState>, filter: EntryFilter) -> CmdResult<Vec<TimeEntry>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    time_entries::list_entries(&conn, &filter)
}

#[tauri::command]
pub fn list_tags(state: State<AppState>) -> CmdResult<Vec<Tag>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tags::list_tags(&conn)
}

#[tauri::command]
pub fn add_tag_to_entry(state: State<AppState>, time_entry_id: i64, tag_name: String) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tags::add_tag_to_entry(&conn, time_entry_id, &tag_name)
}

#[tauri::command]
pub fn remove_tag_from_entry(state: State<AppState>, time_entry_id: i64, tag_name: String) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tags::remove_tag_from_entry(&conn, time_entry_id, &tag_name)
}

#[tauri::command]
pub fn create_tracking_code(
    state: State<AppState>,
    client_id: i64,
    code: String,
    description: Option<String>,
) -> CmdResult<i64> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tracking_codes::create_tracking_code(&conn, client_id, &code, description.as_deref())
}

#[tauri::command]
pub fn list_tracking_codes(
    state: State<AppState>,
    client_id: i64,
    include_archived: bool,
) -> CmdResult<Vec<TrackingCode>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tracking_codes::list_tracking_codes(&conn, client_id, include_archived)
}

#[tauri::command]
pub fn archive_tracking_code(state: State<AppState>, tracking_code_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tracking_codes::archive_tracking_code(&conn, tracking_code_id)
}

#[tauri::command]
pub fn restore_tracking_code(state: State<AppState>, tracking_code_id: i64) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tracking_codes::restore_tracking_code(&conn, tracking_code_id)
}

#[tauri::command]
pub fn list_archived_tracking_codes(state: State<AppState>) -> CmdResult<Vec<TrackingCode>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    tracking_codes::list_all_archived(&conn)
}

#[tauri::command]
pub fn get_user_profile(state: State<AppState>) -> CmdResult<Option<UserProfile>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    user_profile::get_user_profile(&conn)
}

#[tauri::command]
pub fn save_user_profile(state: State<AppState>, profile: UserProfile) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    user_profile::save_user_profile(&conn, &profile)
}

#[derive(serde::Serialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[tauri::command]
pub fn get_window_geometry(window: tauri::WebviewWindow) -> CmdResult<WindowGeometry> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let position = window.outer_position().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    let size = window.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
    Ok(WindowGeometry {
        x: position.x as i32,
        y: position.y as i32,
        width: size.width as i32,
        height: size.height as i32,
    })
}

fn require_output_folder(conn: &rusqlite::Connection) -> CmdResult<String> {
    let profile = user_profile::get_user_profile(conn)?;
    profile
        .and_then(|p| p.output_folder)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "set an output folder in your User profile first".to_string())
}

#[tauri::command]
pub fn generate_report(state: State<AppState>, period: String, reference_date: String) -> CmdResult<Report> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    reports::generate_report(&conn, &period, &reference_date)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn generate_timesheets(
    state: State<AppState>,
    period: String,
    reference_date: String,
    include_rate_amount: bool,
    client_id: Option<i64>,
    contract_id: Option<i64>,
) -> CmdResult<Vec<TimesheetFile>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let profile = user_profile::get_user_profile(&conn)?;
    let profile = profile.ok_or_else(|| "set up your User profile before generating timesheets".to_string())?;
    let output_folder = profile
        .output_folder
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "set an output folder in your User profile before generating timesheets".to_string())?;
    let full_name = profile
        .full_name
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "set your full name in your User profile before generating timesheets".to_string())?;
    timesheets::generate_timesheets(
        &conn,
        &period,
        &reference_date,
        &output_folder,
        &full_name,
        include_rate_amount,
        client_id,
        contract_id,
    )
}

#[tauri::command]
pub fn get_database_size(state: State<AppState>) -> CmdResult<u64> {
    std::fs::metadata(&state.db_path).map(|m| m.len()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_default_output_folder(app: tauri::AppHandle) -> CmdResult<String> {
    app.path()
        .document_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_time_entries(
    state: State<AppState>,
    contract_id: i64,
    file_paths: Vec<String>,
    template_id: i64,
) -> CmdResult<import::ImportResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    import::import_time_entries(&conn, contract_id, &file_paths, template_id)
}

#[tauri::command]
pub fn list_import_templates(state: State<AppState>) -> CmdResult<Vec<ImportTemplate>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    import_templates::list_import_templates(&conn)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_import_template(
    state: State<AppState>,
    name: String,
    notes: Option<String>,
    is_default: bool,
    date_column: String,
    start_column: String,
    end_column: String,
    category_column: Option<String>,
    notes_column: Option<String>,
) -> CmdResult<i64> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    import_templates::create_import_template(
        &conn,
        &name,
        notes.as_deref(),
        is_default,
        &date_column,
        &start_column,
        &end_column,
        category_column.as_deref(),
        notes_column.as_deref(),
    )
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_import_template(
    state: State<AppState>,
    id: i64,
    name: String,
    notes: Option<String>,
    is_default: bool,
    date_column: String,
    start_column: String,
    end_column: String,
    category_column: Option<String>,
    notes_column: Option<String>,
) -> CmdResult<()> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    import_templates::update_import_template(
        &conn,
        id,
        &name,
        notes.as_deref(),
        is_default,
        &date_column,
        &start_column,
        &end_column,
        category_column.as_deref(),
        notes_column.as_deref(),
    )
}

#[tauri::command]
pub fn backup_all_data(state: State<AppState>) -> CmdResult<Vec<BackupFile>> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let output_folder = require_output_folder(&conn)?;
    backup::backup_all_data(&conn, &output_folder)
}

#[tauri::command]
pub fn restore_from_backups(state: State<AppState>, file_paths: Vec<String>) -> CmdResult<RestoreResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    backup::restore_from_backups(&conn, &file_paths)
}

#[tauri::command]
pub fn purge_deleted_entries(state: State<AppState>, cutoff: Option<String>) -> CmdResult<PurgeResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let output_folder = require_output_folder(&conn)?;
    purge::purge_deleted_entries(&conn, cutoff.as_deref(), &output_folder)
}

#[tauri::command]
pub fn purge_archived_contracts(state: State<AppState>, cutoff: Option<String>) -> CmdResult<PurgeResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let output_folder = require_output_folder(&conn)?;
    purge::purge_archived_contracts(&conn, cutoff.as_deref(), &output_folder)
}

#[tauri::command]
pub fn purge_archived_clients(state: State<AppState>, cutoff: Option<String>) -> CmdResult<PurgeResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let output_folder = require_output_folder(&conn)?;
    purge::purge_archived_clients(&conn, cutoff.as_deref(), &output_folder)
}

#[tauri::command]
pub fn purge_archived_categories(state: State<AppState>, cutoff: Option<String>) -> CmdResult<PurgeResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let output_folder = require_output_folder(&conn)?;
    purge::purge_archived_categories(&conn, cutoff.as_deref(), &output_folder)
}

#[tauri::command]
pub fn purge_all_data(state: State<AppState>) -> CmdResult<PurgeAllResult> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let output_folder = require_output_folder(&conn)?;
    purge::purge_all_data(&conn, &output_folder)
}
