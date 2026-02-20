use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

fn default_workspace_count() -> usize {
    10
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub monitor_config_path: String,
    #[serde(default = "default_workspace_count")]
    pub workspace_count: usize,
}

pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    path.to_string()
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| {
        color_eyre::eyre::eyre!("Could not determine config directory")
    })?;
    Ok(base.join("xwlm").join("config.toml"))
}

pub fn load() -> Result<Option<AppConfig>> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(&path).wrap_err("Failed to read config file")?;
    let config: AppConfig =
        toml::from_str(&contents).wrap_err("Failed to parse config file")?;
    Ok(Some(config))
}

pub fn monitor_config_exists(path: &str) -> bool {
    let expanded = expand_tilde(path);
    Path::new(&expanded).exists()
}

pub fn save(config: &AppConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err("Failed to create config directory")?;
    }
    let contents = toml::to_string_pretty(config)
        .wrap_err("Failed to serialize config")?;
    fs::write(&path, contents).wrap_err("Failed to write config file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_tilde() {
        let home = std::env::var("HOME").unwrap_or_default();
        let result = expand_tilde("~/some/path");
        assert_eq!(result, format!("{}/some/path", home));
    }

    #[test]
    fn test_expand_tilde_without_tilde() {
        let result = expand_tilde("/absolute/path");
        assert_eq!(result, "/absolute/path");
    }

    #[test]
    fn test_expand_tilde_empty_string() {
        let result = expand_tilde("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_expand_tilde_just_tilde() {
        let result = expand_tilde("~");
        assert_eq!(result, "~");
    }

    #[test]
    fn test_expand_tilde_tilde_slash_only() {
        let home = std::env::var("HOME").unwrap_or_default();
        let result = expand_tilde("~/");
        assert_eq!(result, format!("{}/", home));
    }

    #[test]
    fn test_monitor_config_exists_with_existing_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_monitors.conf");
        fs::write(&test_file, "test content").unwrap();
        
        let path = test_file.to_string_lossy().to_string();
        assert!(monitor_config_exists(&path));
        
        fs::remove_file(&test_file).ok();
    }

    #[test]
    fn test_monitor_config_exists_with_nonexistent_file() {
        assert!(!monitor_config_exists("/nonexistent/path/to/monitors.conf"));
    }

    #[test]
    fn test_monitor_config_exists_with_tilde() {
        let home = std::env::var("HOME").unwrap_or_default();
        let test_file = std::path::Path::new(&home).join(".test_xwlm_exists.conf");
        let exists_before = test_file.exists();
        
        if !exists_before {
            fs::write(&test_file, "test").unwrap();
        }
        
        assert!(monitor_config_exists("~/.test_xwlm_exists.conf"));
        
        if !exists_before {
            fs::remove_file(&test_file).ok();
        }
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig {
            monitor_config_path: "/home/user/.config/hypr/monitors.conf".to_string(),
            workspace_count: 10,
        };
        
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("monitor_config_path"));
        assert!(toml_str.contains("workspace_count"));
        
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.monitor_config_path, config.monitor_config_path);
        assert_eq!(parsed.workspace_count, config.workspace_count);
    }

    #[test]
    fn test_app_config_default_workspace_count() {
        let toml_str = r#"
monitor_config_path = "/test/path"
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.workspace_count, 10);
    }
}
