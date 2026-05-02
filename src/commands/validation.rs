use crate::commands::descriptor::CommandDescriptor;

pub fn validate_descriptor(descriptor: &CommandDescriptor) -> Result<(), String> {
    if descriptor.id.as_str().trim().is_empty() {
        return Err("command id must not be empty".into());
    }
    if descriptor.title.trim().is_empty() {
        return Err(format!(
            "command {} is missing title",
            descriptor.id.as_str()
        ));
    }
    if descriptor.description.trim().is_empty() {
        return Err(format!(
            "command {} is missing description",
            descriptor.id.as_str()
        ));
    }
    if descriptor.category.trim().is_empty() {
        return Err(format!(
            "command {} is missing category",
            descriptor.id.as_str()
        ));
    }
    if descriptor.examples.is_empty() {
        return Err(format!(
            "command {} must include at least one example",
            descriptor.id.as_str()
        ));
    }

    for argument in &descriptor.arguments {
        if argument.name.trim().is_empty() || argument.description.trim().is_empty() {
            return Err(format!(
                "command {} has invalid argument metadata",
                descriptor.id.as_str()
            ));
        }
    }

    Ok(())
}
