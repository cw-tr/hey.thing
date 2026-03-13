use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub user: UserConfig,
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub somewhere: SomewhereConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SomewhereConfig {
    pub backup: Option<String>,
    pub remote: Option<String>,
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SecurityConfig {
    /// .configthing dosyasının BLAKE3 hash'i. Klonlanan repoda
    /// bu hash eşleşmezse hook'lar devre dışı kalır.
    #[serde(default)]
    pub trusted_config_hash: Option<String>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Mevcut .configthing dosyasının BLAKE3 hash'ini hesaplar
    pub fn compute_hash<P: AsRef<Path>>(path: P) -> Result<String> {
        let content = fs::read(path)?;
        Ok(crate::crypto::hash::hash_data(&content))
    }

    /// Config dosyası güvenilir mi kontrol eder.
    /// trusted_config_hash yoksa → güvenilir (ilk init)
    /// trusted_config_hash varsa → mevcut hash ile karşılaştır
    pub fn is_trusted<P: AsRef<Path>>(path: P) -> Result<bool> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(false);
        }

        let config = Self::load(path)?;
        match config.security.trusted_config_hash {
            None => Ok(true), // Hash kaydı yoksa güvenilir (yerel repo)
            Some(ref stored_hash) => {
                let current_hash = Self::compute_hash(path)?;
                Ok(&current_hash == stored_hash)
            }
        }
    }

    /// Config dosyasını güvenilir olarak işaretle (hash kaydet)
    pub fn mark_trusted<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();
        let hash = Self::compute_hash(path)?;

        let mut config = Self::load(path)?;
        config.security.trusted_config_hash = Some(hash);

        let toml_str = toml::to_string_pretty(&config)?;
        fs::write(path, toml_str)?;

        Ok(())
    }
}
