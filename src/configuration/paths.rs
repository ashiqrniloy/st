use std::{
    env,
    path::{Path, PathBuf},
};

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
