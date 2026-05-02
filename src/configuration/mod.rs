mod defaults;
mod descriptors;
mod paths;
mod registry;
mod schema;
mod state;
mod validation;
mod yaml;

pub use defaults::*;
pub use descriptors::builtin_setting_descriptors;
pub use paths::expand_config_path;
pub use registry::SettingRegistry;
#[allow(unused_imports)]
pub use schema::{
    ConfigSource, JavaScriptConfigApiShape, SettingValue, javascript_config_api_shape,
};
#[allow(unused_imports)]
pub use state::{AppliedSetting, ConfigDiagnostic, ConfigState};
#[allow(unused_imports)]
pub use yaml::{DeclarativeYamlConfig, load_declarative_yaml_file};

#[cfg(test)]
mod tests;
