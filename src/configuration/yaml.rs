use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::configuration::state::ConfigDiagnostic;

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
