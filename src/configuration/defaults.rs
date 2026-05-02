use crate::configuration::schema::SettingValue;

pub const DEFAULT_INIT_JS: &str = "~/.config/st/init.js";
pub const DEFAULT_SETTINGS_YAML: &str = "~/.config/st/settings.yml";
pub const DEFAULT_EXTENSION_DIR: &str = "~/.config/st/extensions";
pub const DEFAULT_TOOL_DIR: &str = "~/.config/st/tools";
pub const RUNTIME_DIR_ENV: &str = "ST_RUNTIME_DIR";
pub const DEFAULT_EDITOR_BACKGROUND_COLOR: u32 = 0x1e1e2e;
pub const DEFAULT_EDITOR_TEXT_COLOR: u32 = 0xcdd6f4;
pub const DEFAULT_SELECTION_COLOR: u32 = 0x3b4261;
pub const DEFAULT_CURSOR_VISIBLE: bool = true;
pub const DEFAULT_AUTO_STARTED_IDLE_TIMEOUT_SECS: u64 = 300;
pub const DEFAULT_CLIENT_WINDOW_WIDTH_PX: u64 = 900;
pub const DEFAULT_CLIENT_WINDOW_HEIGHT_PX: u64 = 600;
pub const DEFAULT_LINE_HEIGHT_PX: f32 = 22.0;
pub const DEFAULT_DWIM_WIDE_ASPECT_RATIO: f32 = 1.7;
pub const DEFAULT_DWIM_HALF_HEIGHT_ASPECT_RATIO: f32 = 1.3;
pub const DEFAULT_DWIM_HALF_HEIGHT_MAX_PX: u32 = 600;

pub const REQUIRED_CONFIGURABLE_SETTING_IDS: &[&str] = &[
    "editor.background_color",
    "editor.cursor_visible",
    "editor.text_color",
    "editor.selection_color",
    "ui.line_height_px",
    "client.window_width_px",
    "client.window_height_px",
    "server.idle_timeout_secs",
    "server.auto_start_idle_timeout_secs",
    "paths.runtime_dir",
    "paths.extension_dirs",
    "paths.tool_dirs",
    "keybindings.default_editor_bindings_enabled",
    "window.split_dwim_wide_aspect_ratio",
    "window.split_dwim_half_height_aspect_ratio",
    "window.split_dwim_half_height_max_px",
];

pub fn default_config_locations() -> Vec<&'static str> {
    vec![
        DEFAULT_INIT_JS,
        DEFAULT_SETTINGS_YAML,
        DEFAULT_EXTENSION_DIR,
        DEFAULT_TOOL_DIR,
    ]
}

pub fn builtin_default_values() -> Vec<(&'static str, SettingValue)> {
    vec![
        (
            "editor.background_color",
            SettingValue::Integer(DEFAULT_EDITOR_BACKGROUND_COLOR as u64),
        ),
        (
            "editor.cursor_visible",
            SettingValue::Bool(DEFAULT_CURSOR_VISIBLE),
        ),
        (
            "editor.text_color",
            SettingValue::Integer(DEFAULT_EDITOR_TEXT_COLOR as u64),
        ),
        (
            "editor.selection_color",
            SettingValue::Integer(DEFAULT_SELECTION_COLOR as u64),
        ),
        (
            "ui.line_height_px",
            SettingValue::FloatString(DEFAULT_LINE_HEIGHT_PX.to_string()),
        ),
        (
            "client.window_width_px",
            SettingValue::Integer(DEFAULT_CLIENT_WINDOW_WIDTH_PX),
        ),
        (
            "client.window_height_px",
            SettingValue::Integer(DEFAULT_CLIENT_WINDOW_HEIGHT_PX),
        ),
        ("server.idle_timeout_secs", SettingValue::None),
        (
            "server.auto_start_idle_timeout_secs",
            SettingValue::Integer(DEFAULT_AUTO_STARTED_IDLE_TIMEOUT_SECS),
        ),
        (
            "paths.runtime_dir",
            SettingValue::String("$XDG_RUNTIME_DIR/st or temp/st".into()),
        ),
        ("paths.extension_dirs", SettingValue::StringList(vec![])),
        ("paths.tool_dirs", SettingValue::StringList(vec![])),
        (
            "keybindings.default_editor_bindings_enabled",
            SettingValue::Bool(true),
        ),
        (
            "window.split_dwim_wide_aspect_ratio",
            SettingValue::FloatString(DEFAULT_DWIM_WIDE_ASPECT_RATIO.to_string()),
        ),
        (
            "window.split_dwim_half_height_aspect_ratio",
            SettingValue::FloatString(DEFAULT_DWIM_HALF_HEIGHT_ASPECT_RATIO.to_string()),
        ),
        (
            "window.split_dwim_half_height_max_px",
            SettingValue::Integer(DEFAULT_DWIM_HALF_HEIGHT_MAX_PX as u64),
        ),
    ]
}
