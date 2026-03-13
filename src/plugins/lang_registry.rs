use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use crate::plugins::wasm_engine::WasmLangPlugin;
use crate::core::ast_plugin::LangPlugin;

pub struct LangRegistry {
    plugins: HashMap<String, Arc<dyn LangPlugin>>,
}

impl LangRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Arc<dyn LangPlugin>) {
        for ext in plugin.extensions() {
            self.plugins.insert(ext, plugin.clone());
        }
    }

    pub fn get_merger(&self, path: &str) -> Option<Arc<dyn LangPlugin>> {
        let ext = std::path::Path::new(path).extension()?.to_str()?;
        self.plugins.get(ext).cloned()
    }

    /// `~/.something/langs/` dizinindeki *.thing (WASM) dosyalarını tarayıp yükler.
    pub fn load_plugins_from_dir(&mut self, dir_path: &Path) {
        if !dir_path.exists() { return; }

        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("thing") {
                    // Güvenlik: İleride burada signature verification (imza doğrulaması) yapılacak.
                    match WasmLangPlugin::new(&path) {
                        Ok(plugin) => {
                            self.register(Arc::new(plugin));
                        }
                        Err(e) => {
                            eprintln!("Hata: {} yüklenemedi -> {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }
}
