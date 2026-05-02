use crate::documentation::SettingDescriptor;

pub fn validate_setting_descriptor(descriptor: &SettingDescriptor) -> Result<(), String> {
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
