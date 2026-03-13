use crate::core::verb_plugin::VerbPlugin;
use std::collections::HashMap;

const PROTECTED: &[&str] = &[
    "save", "shift", "sync", "undo", "rewind", "show", "init", "get", "branch", "import", "verb",
    "lang", "plugin", "help", "setup",
];

pub fn is_protected(name: &str) -> bool {
    PROTECTED.contains(&name)
}

pub struct VerbRegistry {
    verbs: HashMap<String, Box<dyn VerbPlugin>>,
}

impl VerbRegistry {
    pub fn new() -> Self {
        Self {
            verbs: HashMap::new(),
        }
    }

    pub fn register(&mut self, verb: Box<dyn VerbPlugin>) {
        if PROTECTED.contains(&verb.name()) {
            // Built-in'leri korumalı listeye rağmen kaydedebiliriz (main.rs içinde)
            // Ama dışarıdan plugin yüklerken bu kontrol hayati olacak.
        }
        self.verbs.insert(verb.name().to_string(), verb);
    }

    pub fn find(&self, name: &str) -> Option<&dyn VerbPlugin> {
        self.verbs.get(name).map(|v| v.as_ref()).or_else(|| {
            self.verbs
                .values()
                .find(|v| v.aliases().contains(&name))
                .map(|v| v.as_ref())
        })
    }

    pub fn list_help(&self) {
        println!("hey.thing - Kullanılabilir komutlar:\n");
        for verb in self.verbs.values() {
            println!("  {: <10} {}", verb.name(), verb.help());
        }
    }

    /// Verilen dizinlerdeki *.thing (WASM) dosyalarını tarayıp yükler.
    pub fn load_plugins_from_dirs(&mut self, dirs: &[std::path::PathBuf]) {
        for dir_path in dirs {
            if !dir_path.exists() { continue; }

            if let Ok(entries) = std::fs::read_dir(dir_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("thing") {
                        let verb_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        
                        // Korumalı komut isimlerinin eklenti tarafından ezilmesi engellenir
                        if PROTECTED.contains(&verb_name.as_str()) {
                            eprintln!("[!] Uyarı: '{}' komutu core protect listesindedir. Eklenti yüklenmedi: {}", verb_name, path.display());
                            continue;
                        }

                        // Eğer bu komut zaten kaydedilmişse (üst katmandan gelen) atla
                        if self.verbs.contains_key(&verb_name) {
                            continue;
                        }
                        
                        // TODO: WasmVerbPlugin eklendiğinde burada doğrudan self.register(...) yapılacak.
                        // eprintln!("Bilgi: WASM Verb Plugin ({}) bulundu, motor tam hazır olmadığından atlanıyor.", verb_name);
                    }
                }
            }
        }
    }
}
