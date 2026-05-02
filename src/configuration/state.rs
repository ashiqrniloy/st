use std::{collections::HashMap, path::PathBuf};

use crate::configuration::{
    defaults::builtin_default_values,
    registry::SettingRegistry,
    schema::{ConfigSource, SettingValue},
};

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
