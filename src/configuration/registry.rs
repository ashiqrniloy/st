use std::collections::HashMap;

use crate::{
    configuration::{
        descriptors::builtin_setting_descriptors, validation::validate_setting_descriptor,
    },
    documentation::SettingDescriptor,
};

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
