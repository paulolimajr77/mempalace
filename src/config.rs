use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Global mempalace configuration (~/.mempalace/config.json).
#[derive(Debug, Serialize, Deserialize)]
pub struct MempalaceConfig {
    #[serde(default = "default_palace_path")]
    pub palace_path: PathBuf,

    #[serde(default = "default_collection_name")]
    pub collection_name: String,

    #[serde(default)]
    pub people_map: HashMap<String, String>,
}

fn default_palace_path() -> PathBuf {
    config_dir().join("palace.db")
}

fn default_collection_name() -> String {
    "mempalace_drawers".to_string()
}

/// Returns ~/.mempalace/
pub fn config_dir() -> PathBuf {
    dirs_fallback().join(".mempalace")
}

/// Returns the user's home directory.
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from)
}

/// Path to the global config file.
pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

impl MempalaceConfig {
    /// Load config from ~/.mempalace/config.json, or return defaults.
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let cfg: Self = serde_json::from_str(&data)?;
            Ok(cfg)
        } else {
            Ok(Self::default())
        }
    }

    /// Ensure the config directory and default config exist.
    pub fn init() -> Result<Self> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;

        let path = config_path();
        if path.exists() {
            Self::load()
        } else {
            let cfg = Self::default();
            let data = serde_json::to_string_pretty(&cfg)?;
            std::fs::write(&path, data)?;
            Ok(cfg)
        }
    }

    /// Resolve the palace database path, respecting `MEMPALACE_PALACE_PATH` env var.
    pub fn palace_db_path(&self) -> PathBuf {
        if let Ok(env_path) = std::env::var("MEMPALACE_PALACE_PATH") {
            return PathBuf::from(env_path);
        }
        self.palace_path.clone()
    }
}

impl Default for MempalaceConfig {
    fn default() -> Self {
        Self {
            palace_path: default_palace_path(),
            collection_name: default_collection_name(),
            people_map: HashMap::new(),
        }
    }
}

/// Per-project config (mempalace.yaml).
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub wing: String,
    pub rooms: Vec<RoomConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomConfig {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl ProjectConfig {
    /// Load from a mempalace.yaml file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::ConfigNotFound(path.to_path_buf()));
        }
        let data = std::fs::read_to_string(path)?;
        let cfg: Self = serde_yaml::from_str(&data)?;
        Ok(cfg)
    }
}
