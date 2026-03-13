pub mod hooks_api;
pub mod lang_registry;
pub mod verb_registry;
pub mod wasm_engine;

use std::path::PathBuf;
use directories::UserDirs;

/// `~/.something` dizinini döndürür.
pub fn get_something_dir() -> PathBuf {
    if let Some(user_dirs) = UserDirs::new() {
        let mut path = user_dirs.home_dir().to_path_buf();
        path.push(".something");
        path
    } else {
        PathBuf::from(".something") // Fallback
    }
}
