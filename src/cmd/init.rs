use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::Result;
use std::fs;
use std::path::Path;
use crate::crypto::auth::KeyManager;

pub struct InitVerb;

impl InitVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for InitVerb {
    fn name(&self) -> &str {
        "init"
    }

    fn aliases(&self) -> &[&str] {
        &[]
    }

    fn help(&self) -> &str {
        "Yeni bir .something reposu oluşturur"
    }

    fn run(&self, _ctx: &ThingContext, _args: &[String]) -> Result<()> {
        let something_dir = Path::new(".something");
        let config_file = Path::new(".configthing");

        if something_dir.exists() {
            println!(".something dizini zaten mevcut. Yeniden başlatılmadı.");
            return Ok(());
        }

        fs::create_dir(something_dir)?;
        println!(".something dizini oluşturuldu.");

        if !config_file.exists() {
            let default_config = r#"[user]
name = "Anonymous"

[behavior]
auto_stage_all = true
ignore_empty_commits = true
journal_retention = "90d"

[somewhere]
backup = "/tmp/hey-thing-backup"
# remote = "http://somewhere.cw.tr/mukan/hey-thing"
"#;
            fs::write(config_file, default_config)?;
            println!(".configthing dosyası oluşturuldu.");
        }

        // Anahtar kontrolü/üretimi
        match KeyManager::get_or_create_key() {
            Ok(key) => {
                let pub_key = hex::encode(key.verifying_key().to_bytes());
                println!("Kimlik doğrulama anahtarları hazır.");
                println!("Public Key: {}", pub_key);
            },
            Err(e) => println!("Uyarı: Anahtar üretilemedi: {}", e),
        }

        println!("hey.thing reposu başarıyla oluşturuldu.");
        Ok(())
    }
}
