use anyhow::{Context, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
}

impl ConfigFormat {
    pub fn from_path(path: &Path) -> Result<Self> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml" | "yml") => Ok(Self::Yaml),
            Some(ext) => anyhow::bail!("Unsupported config extension '.{}'", ext),
            None => anyhow::bail!("Config file has no extension: {}", path.display()),
        }
    }

    pub fn is_supported_path(path: &Path) -> bool {
        matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("toml" | "yaml" | "yml")
        )
    }
}

pub fn from_str<T>(content: &str, format: ConfigFormat) -> Result<T>
where
    T: DeserializeOwned,
{
    match format {
        ConfigFormat::Toml => toml::from_str(content).context("Failed to parse TOML"),
        ConfigFormat::Yaml => yaml_serde::from_str(content).context("Failed to parse YAML"),
    }
}

pub fn to_string<T>(value: &T, format: ConfigFormat) -> Result<String>
where
    T: Serialize,
{
    match format {
        ConfigFormat::Toml => toml::to_string_pretty(value).context("Failed to serialize TOML"),
        ConfigFormat::Yaml => yaml_serde::to_string(value).context("Failed to serialize YAML"),
    }
}
