use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::DomainResult;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub output_folder: Option<String>,
    pub output_type: String,
    pub window_width: Option<i64>,
    pub window_height: Option<i64>,
    pub window_x: Option<i64>,
    pub window_y: Option<i64>,
    pub default_start_page: String,
    pub launch_position: String,
    pub theme: String,
    pub custom_bg: Option<String>,
    pub custom_text: Option<String>,
    pub custom_button_bg: Option<String>,
    pub custom_button_text: Option<String>,
}

pub fn get_user_profile(conn: &Connection) -> DomainResult<Option<UserProfile>> {
    conn.query_row(
        "SELECT first_name, last_name, full_name, email, output_folder, output_type,
                window_width, window_height, window_x, window_y, default_start_page, launch_position,
                theme, custom_bg, custom_text, custom_button_bg, custom_button_text
         FROM user_profile WHERE id = 1",
        [],
        |row| {
            Ok(UserProfile {
                first_name: row.get(0)?,
                last_name: row.get(1)?,
                full_name: row.get(2)?,
                email: row.get(3)?,
                output_folder: row.get(4)?,
                output_type: row.get(5)?,
                window_width: row.get(6)?,
                window_height: row.get(7)?,
                window_x: row.get(8)?,
                window_y: row.get(9)?,
                default_start_page: row.get(10)?,
                launch_position: row.get(11)?,
                theme: row.get(12)?,
                custom_bg: row.get(13)?,
                custom_text: row.get(14)?,
                custom_button_bg: row.get(15)?,
                custom_button_text: row.get(16)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn save_user_profile(conn: &Connection, profile: &UserProfile) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO user_profile (id, first_name, last_name, full_name, email, output_folder, output_type,
            window_width, window_height, window_x, window_y, default_start_page, launch_position,
            theme, custom_bg, custom_text, custom_button_bg, custom_button_text)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
         ON CONFLICT(id) DO UPDATE SET
            first_name = excluded.first_name,
            last_name = excluded.last_name,
            full_name = excluded.full_name,
            email = excluded.email,
            output_folder = excluded.output_folder,
            output_type = excluded.output_type,
            window_width = excluded.window_width,
            window_height = excluded.window_height,
            window_x = excluded.window_x,
            window_y = excluded.window_y,
            default_start_page = excluded.default_start_page,
            launch_position = excluded.launch_position,
            theme = excluded.theme,
            custom_bg = excluded.custom_bg,
            custom_text = excluded.custom_text,
            custom_button_bg = excluded.custom_button_bg,
            custom_button_text = excluded.custom_button_text",
        params![
            profile.first_name,
            profile.last_name,
            profile.full_name,
            profile.email,
            profile.output_folder,
            profile.output_type,
            profile.window_width,
            profile.window_height,
            profile.window_x,
            profile.window_y,
            profile.default_start_page,
            profile.launch_position,
            profile.theme,
            profile.custom_bg,
            profile.custom_text,
            profile.custom_button_bg,
            profile.custom_button_text,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
