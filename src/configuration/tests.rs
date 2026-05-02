use std::path::{Path, PathBuf};

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
fn exposes_javascript_config_shape_without_mandatory_paths() {
    let api = javascript_config_api_shape();
    assert_eq!(api.primary_entrypoint, DEFAULT_INIT_JS);
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
