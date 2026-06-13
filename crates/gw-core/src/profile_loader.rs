use crate::policy::PolicyProfile;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

pub struct ProfileLoader;

impl ProfileLoader {
    pub fn new() -> Self {
        Self
    }

    /// Load policy profile from TOML or YAML file.
    pub fn load_profile(&self, path: &Path) -> Result<PolicyProfile> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read policy profile from {:?}", path))?;
        let format = crate::config_format::ConfigFormat::from_path(path)?;

        let profile: PolicyProfile = crate::config_format::from_str(&content, format)?;

        Ok(profile)
    }

    /// Load all policy profiles from a directory
    pub fn load_profiles_from_dir(&self, dir: &Path) -> Result<HashMap<String, PolicyProfile>> {
        let mut profiles = HashMap::new();

        if !dir.exists() {
            return Ok(profiles);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if crate::config_format::ConfigFormat::is_supported_path(&path) {
                match self.load_profile(&path) {
                    Ok(profile) => {
                        profiles.insert(profile.name.clone(), profile);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load profile from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(profiles)
    }

    /// Load default profiles
    pub fn load_default_profiles(&self) -> HashMap<String, PolicyProfile> {
        let mut profiles = HashMap::new();

        // Try to load from examples/policies
        if let Ok(loaded) = self.load_profiles_from_dir(Path::new("examples/policies")) {
            profiles.extend(loaded);
        }

        // Try to load from /etc/gwarden/policies
        if let Ok(loaded) = self.load_profiles_from_dir(Path::new("/etc/gwarden/policies")) {
            profiles.extend(loaded);
        }

        profiles
    }
}

impl Default for ProfileLoader {
    fn default() -> Self {
        Self::new()
    }
}
