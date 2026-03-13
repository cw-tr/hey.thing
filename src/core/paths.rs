use std::path::PathBuf;
use std::fs;

pub fn get_global_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".something")
}

pub fn get_global_keys_dir() -> PathBuf {
    get_global_dir().join("keys")
}

pub fn get_default_key_path() -> PathBuf {
    get_global_keys_dir().join("id_thing_ed25519")
}

pub fn ensure_global_dirs() -> std::io::Result<()> {
    let keys_dir = get_global_keys_dir();
    if !keys_dir.exists() {
        fs::create_dir_all(keys_dir)?;
    }
    Ok(())
}
