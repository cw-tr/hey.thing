pub mod hooks_api;
pub mod lang_registry;
pub mod verb_registry;
pub mod wasm_engine;

use std::path::PathBuf;
use directories::UserDirs;
use std::env;

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

/// Eklentilerin aranacağı dizinleri liste olarak döndürür.
/// Sıralama: [User Layer, Binary Layer]
pub fn get_plugin_search_paths(sub_dir: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. User Layer (~/.something/...) - En yüksek öncelik
    paths.push(get_something_dir().join(sub_dir));

    // 2. Binary Layer (hey binary'sinin yanındaki klasör)
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths.push(exe_dir.join(sub_dir));
        }
    }

    // 3. (Opsiyonel) Dev Layer: Proje kök dizinindeki target veya plugins klasörü de eklenebilir
    // Ama şimdilik temiz tutalım.

    paths
}
