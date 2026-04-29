use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::documentation::SettingDescriptor;

pub const DEFAULT_INIT_TS: &str = "~/.config/st/init.ts";
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
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigSource {
    Default,
    Cli,
    InitTs,
    Yaml,
    Extension,
    RuntimeOverride,
}

impl ConfigSource {
    pub fn precedence(self) -> u8 {
        match self {
            Self::Default => 0,
            Self::Yaml => 10,
            Self::InitTs => 20,
            Self::Extension => 30,
            Self::Cli => 40,
            Self::RuntimeOverride => 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingValue {
    Bool(bool),
    Integer(u64),
    String(String),
    StringList(Vec<String>),
    FloatString(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedSetting {
    pub value: SettingValue,
    pub source: ConfigSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub message: String,
    pub source: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct SettingRegistry {
    settings: HashMap<String, SettingDescriptor>,
}

impl SettingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_settings() -> Result<Self, String> {
        let mut registry = Self::new();
        for descriptor in builtin_setting_descriptors() {
            registry.register(descriptor)?;
        }
        Ok(registry)
    }

    pub fn register(&mut self, descriptor: SettingDescriptor) -> Result<(), String> {
        validate_setting_descriptor(&descriptor)?;
        if self.settings.contains_key(&descriptor.id) {
            return Err(format!("setting id already registered: {}", descriptor.id));
        }
        self.settings.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&SettingDescriptor> {
        self.settings.get(id)
    }

    pub fn list(&self) -> Vec<SettingDescriptor> {
        let mut settings: Vec<_> = self.settings.values().cloned().collect();
        settings.sort_by(|a, b| a.id.cmp(&b.id));
        settings
    }
}

#[derive(Debug)]
pub struct ConfigState {
    registry: SettingRegistry,
    values: HashMap<String, AppliedSetting>,
}

impl ConfigState {
    pub fn new(registry: SettingRegistry) -> Self {
        Self {
            registry,
            values: HashMap::new(),
        }
    }

    pub fn with_builtin_defaults() -> Result<Self, ConfigDiagnostic> {
        let registry =
            SettingRegistry::with_builtin_settings().map_err(|err| ConfigDiagnostic {
                message: err,
                source: None,
            })?;
        let mut state = Self::new(registry);
        for (id, value) in builtin_default_values() {
            state.apply_setting(id, value, ConfigSource::Default)?;
        }
        Ok(state)
    }

    pub fn apply_setting(
        &mut self,
        id: &str,
        value: SettingValue,
        source: ConfigSource,
    ) -> Result<(), ConfigDiagnostic> {
        if self.registry.get(id).is_none() {
            return Err(ConfigDiagnostic {
                message: format!("unknown setting id: {id}"),
                source: None,
            });
        }

        let should_apply = self
            .values
            .get(id)
            .map(|current| source.precedence() >= current.source.precedence())
            .unwrap_or(true);

        if should_apply {
            self.values
                .insert(id.into(), AppliedSetting { value, source });
        }

        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&AppliedSetting> {
        self.values.get(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeScriptConfigApiShape {
    pub primary_entrypoint: &'static str,
    pub module_name: &'static str,
    pub methods: &'static [&'static str],
}

pub fn typescript_config_api_shape() -> TypeScriptConfigApiShape {
    TypeScriptConfigApiShape {
        primary_entrypoint: DEFAULT_INIT_TS,
        module_name: "st",
        methods: &[
            "config.loadYaml(path)",
            "editor.set(settings)",
            "extensions.load(path)",
            "extensions.loadDir(path)",
            "tools.load(path)",
            "tools.loadDir(path)",
        ],
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeclarativeYamlConfig {
    #[serde(default)]
    pub editor: serde_yaml::Value,
    #[serde(default)]
    pub extension_dirs: Vec<String>,
    #[serde(default)]
    pub tool_dirs: Vec<String>,
}

#[allow(dead_code)]
pub fn load_declarative_yaml_file(path: &Path) -> Result<DeclarativeYamlConfig, ConfigDiagnostic> {
    let text = fs::read_to_string(path).map_err(|err| ConfigDiagnostic {
        message: format!("failed to read YAML config: {err}"),
        source: Some(path.to_path_buf()),
    })?;

    serde_yaml::from_str(&text).map_err(|err| ConfigDiagnostic {
        message: format!("invalid declarative YAML config: {err}"),
        source: Some(path.to_path_buf()),
    })
}

pub fn expand_config_path(path: &str, base_dir: Option<&Path>) -> PathBuf {
    let home = env::var_os("HOME").map(PathBuf::from);
    let expanded_home = if path == "~" {
        home.clone().unwrap_or_else(|| PathBuf::from(path))
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.clone()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(expand_environment_variables(path))
    };

    if expanded_home.is_relative() {
        base_dir
            .unwrap_or_else(|| Path::new("."))
            .join(expanded_home)
    } else {
        expanded_home
    }
}

pub fn default_config_locations() -> Vec<&'static str> {
    vec![
        DEFAULT_INIT_TS,
        DEFAULT_SETTINGS_YAML,
        DEFAULT_EXTENSION_DIR,
        DEFAULT_TOOL_DIR,
    ]
}

pub fn builtin_setting_descriptors() -> Vec<SettingDescriptor> {
    vec![
        setting(
            "editor.background_color",
            "Editor Background Color",
            "Background color used for editor scenes.",
            "u32 RGB hex",
            &format!("0x{DEFAULT_EDITOR_BACKGROUND_COLOR:06x}"),
            &["Any 24-bit RGB value"],
            &["editor.set({ backgroundColor: 0x1e1e2e })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies to newly generated scene updates",
        ),
        setting(
            "editor.cursor_visible",
            "Cursor Visible",
            "Whether the editor cursor is shown by default.",
            "bool",
            if DEFAULT_CURSOR_VISIBLE {
                "true"
            } else {
                "false"
            },
            &["true", "false"],
            &["editor.set({ cursorVisible: true })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies to newly generated scene updates",
        ),
        setting(
            "editor.text_color",
            "Editor Text Color",
            "Default text color used by the native editor surface.",
            "u32 RGB hex",
            &format!("0x{DEFAULT_EDITOR_TEXT_COLOR:06x}"),
            &["Any 24-bit RGB value"],
            &["editor.set({ textColor: 0xcdd6f4 })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "requires repaint; runtime application is future work",
        ),
        setting(
            "editor.selection_color",
            "Selection Color",
            "Default selected-text highlight color.",
            "u32 RGB hex",
            &format!("0x{DEFAULT_SELECTION_COLOR:06x}"),
            &["Any 24-bit RGB value"],
            &["editor.set({ selectionColor: 0x3b4261 })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "requires repaint; runtime application is future work",
        ),
        setting(
            "ui.line_height_px",
            "Editor Line Height",
            "Native editor line height in pixels.",
            "f32 pixels",
            &DEFAULT_LINE_HEIGHT_PX.to_string(),
            &["Positive pixel value"],
            &["editor.set({ lineHeightPx: 22 })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies when editor views are created or re-rendered",
        ),
        setting(
            "client.window_width_px",
            "Client Window Width",
            "Default client window width in pixels.",
            "u64 pixels",
            &DEFAULT_CLIENT_WINDOW_WIDTH_PX.to_string(),
            &["Positive integer pixels"],
            &["editor.set({ windowWidthPx: 900 })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies when a client window is opened",
        ),
        setting(
            "client.window_height_px",
            "Client Window Height",
            "Default client window height in pixels.",
            "u64 pixels",
            &DEFAULT_CLIENT_WINDOW_HEIGHT_PX.to_string(),
            &["Positive integer pixels"],
            &["editor.set({ windowHeightPx: 600 })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies when a client window is opened",
        ),
        setting(
            "server.idle_timeout_secs",
            "Server Idle Timeout",
            "When no clients remain, an explicitly started server exits after this timeout when configured.",
            "Option<u64>",
            "None for explicit `st server`",
            &["None", "Any non-negative integer seconds"],
            &[
                "st server --idle-timeout-secs 300",
                "st server --no-idle-timeout",
            ],
            &[
                "CLI flag --idle-timeout-secs",
                "CLI flag --no-idle-timeout",
                "future init.ts API",
            ],
            "requires server restart until runtime config reload exists",
        ),
        setting(
            "server.auto_start_idle_timeout_secs",
            "Auto-start Server Idle Timeout",
            "Idle timeout used when a client auto-starts the server.",
            "u64 seconds",
            &DEFAULT_AUTO_STARTED_IDLE_TIMEOUT_SECS.to_string(),
            &["Any non-negative integer seconds"],
            &["config.set('server.auto_start_idle_timeout_secs', 300)"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies before auto-starting a server process",
        ),
        setting(
            "paths.runtime_dir",
            "Runtime Directory",
            "Directory containing runtime IPC files such as the server socket.",
            "Path",
            "$XDG_RUNTIME_DIR/st or system temporary directory + /st",
            &["Any user-writable directory path"],
            &["ST_RUNTIME_DIR=/tmp/st-runtime st"],
            &[
                "ST_RUNTIME_DIR bootstrap environment variable",
                "future init.ts API",
            ],
            "must be known before client/server IPC starts",
        ),
        setting(
            "paths.extension_dirs",
            "Extension Directories",
            "Directories that user configuration may explicitly load extensions from.",
            "Vec<Path>",
            "[]; conventional path ~/.config/st/extensions is not mandatory",
            &["Any readable directory path chosen by the user"],
            &["await extensions.loadDir(\"~/work/editor-extensions\")"],
            &["init.ts", "YAML field extension_dirs consumed by init.ts"],
            "applies when config/extensions are loaded or reloaded",
        ),
        setting(
            "paths.tool_dirs",
            "Tool Directories",
            "Directories that user configuration may explicitly load tools from.",
            "Vec<Path>",
            "[]; conventional path ~/.config/st/tools is not mandatory",
            &["Any readable directory path chosen by the user"],
            &["await tools.loadDir(\"~/.config/st/tools\")"],
            &["init.ts", "YAML field tool_dirs consumed by init.ts"],
            "applies when config/tools are loaded or reloaded",
        ),
        setting(
            "keybindings.default_editor_bindings_enabled",
            "Default Editor Keybindings Enabled",
            "Whether built-in GPUI editor keybindings are installed by default before the future keymap registry takes over.",
            "bool",
            "true",
            &["true", "false"],
            &["keymap.defaults({ editor: true })"],
            &["future init.ts API", "YAML loaded explicitly by init.ts"],
            "applies when a client process starts",
        ),
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
    ]
}

fn setting(
    id: &str,
    title: &str,
    description: &str,
    value_type: &str,
    default_value: &str,
    valid_values: &[&str],
    examples: &[&str],
    configured_via: &[&str],
    reload_behavior: &str,
) -> SettingDescriptor {
    SettingDescriptor {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        value_type: value_type.into(),
        default_value: default_value.into(),
        valid_values: valid_values.iter().map(|value| (*value).into()).collect(),
        examples: examples.iter().map(|value| (*value).into()).collect(),
        configured_via: configured_via.iter().map(|value| (*value).into()).collect(),
        reload_behavior: reload_behavior.into(),
    }
}

fn validate_setting_descriptor(descriptor: &SettingDescriptor) -> Result<(), String> {
    if descriptor.id.trim().is_empty() {
        return Err("setting id must not be empty".into());
    }
    if descriptor.title.trim().is_empty() {
        return Err(format!("setting {} is missing title", descriptor.id));
    }
    if descriptor.description.trim().is_empty() {
        return Err(format!("setting {} is missing description", descriptor.id));
    }
    if descriptor.value_type.trim().is_empty() {
        return Err(format!("setting {} is missing type", descriptor.id));
    }
    if descriptor.default_value.trim().is_empty() {
        return Err(format!("setting {} is missing default", descriptor.id));
    }
    if descriptor.examples.is_empty() {
        return Err(format!("setting {} must include an example", descriptor.id));
    }
    if descriptor.configured_via.is_empty() {
        return Err(format!(
            "setting {} must document configuration path",
            descriptor.id
        ));
    }
    if descriptor.reload_behavior.trim().is_empty() {
        return Err(format!(
            "setting {} is missing reload behavior",
            descriptor.id
        ));
    }
    Ok(())
}

fn expand_environment_variables(path: &str) -> String {
    let mut output = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        if chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            for next in chars.by_ref() {
                if next == '}' {
                    break;
                }
                name.push(next);
            }
            output.push_str(&env::var(&name).unwrap_or_default());
        } else {
            let mut name = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_alphanumeric() || next == '_' {
                    name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if name.is_empty() {
                output.push('$');
            } else {
                output.push_str(&env::var(&name).unwrap_or_default());
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_descriptor_validation_rejects_missing_docs() {
        let mut registry = SettingRegistry::new();
        let mut descriptor = builtin_setting_descriptors().remove(0);
        descriptor.description.clear();
        let err = registry
            .register(descriptor)
            .expect_err("invalid descriptor");
        assert!(err.contains("missing description"));
    }

    #[test]
    fn builtin_settings_have_required_documentation() {
        let registry = SettingRegistry::with_builtin_settings().expect("builtin settings");
        for setting in registry.list() {
            assert!(!setting.id.trim().is_empty());
            assert!(!setting.default_value.trim().is_empty());
            assert!(!setting.configured_via.is_empty());
            assert!(!setting.reload_behavior.trim().is_empty());
        }
    }

    #[test]
    fn all_current_configurable_options_have_defaults_and_docs() {
        let registry = SettingRegistry::with_builtin_settings().expect("builtin settings");
        for required in REQUIRED_CONFIGURABLE_SETTING_IDS {
            let descriptor = registry.get(required).expect("required setting descriptor");
            assert!(!descriptor.default_value.trim().is_empty());
            assert!(!descriptor.description.trim().is_empty());
            assert!(!descriptor.examples.is_empty());
        }
    }

    #[test]
    fn config_precedence_keeps_higher_precedence_values() {
        let registry = SettingRegistry::with_builtin_settings().expect("builtin settings");
        let mut state = ConfigState::new(registry);
        state
            .apply_setting(
                "server.idle_timeout_secs",
                SettingValue::Integer(300),
                ConfigSource::Default,
            )
            .expect("default applies");
        state
            .apply_setting(
                "server.idle_timeout_secs",
                SettingValue::Integer(10),
                ConfigSource::Cli,
            )
            .expect("cli applies");
        state
            .apply_setting(
                "server.idle_timeout_secs",
                SettingValue::Integer(20),
                ConfigSource::Yaml,
            )
            .expect("lower precedence ignored");

        assert_eq!(
            state.get("server.idle_timeout_secs").map(|s| &s.value),
            Some(&SettingValue::Integer(10))
        );
    }

    #[test]
    fn configured_values_override_builtin_defaults() {
        let mut state = ConfigState::with_builtin_defaults().expect("builtin defaults");
        assert_eq!(
            state.get("editor.background_color").map(|s| &s.value),
            Some(&SettingValue::Integer(
                DEFAULT_EDITOR_BACKGROUND_COLOR as u64
            ))
        );

        state
            .apply_setting(
                "editor.background_color",
                SettingValue::Integer(0x112233),
                ConfigSource::Yaml,
            )
            .expect("configured value applies");

        assert_eq!(
            state.get("editor.background_color").map(|s| &s.value),
            Some(&SettingValue::Integer(0x112233))
        );
    }

    #[test]
    fn invalid_setting_returns_diagnostic() {
        let registry = SettingRegistry::with_builtin_settings().expect("builtin settings");
        let mut state = ConfigState::new(registry);
        let err = state
            .apply_setting(
                "missing.setting",
                SettingValue::Bool(true),
                ConfigSource::Yaml,
            )
            .expect_err("unknown setting should fail");
        assert!(err.message.contains("unknown setting id"));
    }

    #[test]
    fn exposes_typescript_first_config_shape_without_mandatory_paths() {
        let api = typescript_config_api_shape();
        assert_eq!(api.primary_entrypoint, DEFAULT_INIT_TS);
        assert!(api.methods.contains(&"config.loadYaml(path)"));
        assert!(default_config_locations().contains(&DEFAULT_EXTENSION_DIR));
    }

    #[test]
    fn parses_declarative_yaml_data_shape() {
        let yaml = "extension_dirs:\n  - ~/.config/st/extensions\ntool_dirs:\n  - ~/tools\n";
        let parsed: DeclarativeYamlConfig = serde_yaml::from_str(yaml).expect("valid yaml");
        assert_eq!(parsed.extension_dirs, vec!["~/.config/st/extensions"]);
        assert_eq!(parsed.tool_dirs, vec!["~/tools"]);
    }

    #[test]
    fn expands_relative_paths_against_base_dir() {
        let expanded = expand_config_path("extensions", Some(Path::new("/tmp/st")));
        assert_eq!(expanded, PathBuf::from("/tmp/st/extensions"));
    }
}
