use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub user: UserConfig,
    pub behavior: BehaviorConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserConfig {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BehaviorConfig {
    pub auto_stage_all: bool,
    pub ignore_empty_commits: bool,
    pub journal_retention: String,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
