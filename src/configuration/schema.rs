use serde::{Deserialize, Serialize};

use crate::configuration::defaults::DEFAULT_INIT_JS;

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
pub struct JavaScriptConfigApiShape {
    pub primary_entrypoint: &'static str,
    pub module_name: &'static str,
    pub methods: &'static [&'static str],
}

pub fn javascript_config_api_shape() -> JavaScriptConfigApiShape {
    JavaScriptConfigApiShape {
        primary_entrypoint: DEFAULT_INIT_JS,
        module_name: "st",
        methods: &[
            "config.loadYaml(path)",
            "editor.set(settings)",
            "extensions.load(path)",
            "extensions.loadDir(path)",
            "tools.load(path)",
            "tools.loadDir(path)",
            "keymap.bind(chord, commandId)",
        ],
    }
}
