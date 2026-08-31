pub mod backup;
pub mod contracts;
pub mod import;
pub mod import_templates;
pub mod period;
pub mod purge;
pub mod reports;
pub mod tags;
pub mod time_entries;
pub mod timesheets;
pub mod tracking_codes;
pub mod user_profile;

/// Domain functions return `Result<T, String>` so errors can cross the Tauri IPC
/// boundary directly (command handlers just need `.map_err(|e| e.to_string())` at the
/// rusqlite call sites, or propagate this alias's `String` errors as-is).
pub type DomainResult<T> = std::result::Result<T, String>;
