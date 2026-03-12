use crate::core::verb_plugin::VerbPlugin;
use std::collections::HashMap;

const PROTECTED: &[&str] = &[
    "save", "shift", "sync", "undo", "rewind", "show", "init", "get", "branch", "import", "verb",
    "plugin", "help",
];

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
}
