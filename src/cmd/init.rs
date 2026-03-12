use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::Result;
use std::fs;
use std::path::Path;

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
# crypto_key = "~/.something/keys/id_thing_ed25519"

[behavior]
auto_stage_all = true
ignore_empty_commits = true
journal_retention = "90d"
"#;
            fs::write(config_file, default_config)?;
            println!(".configthing dosyası oluşturuldu.");
        }

        println!("hey.thing reposu başarıyla oluşturuldu.");
        Ok(())
    }
}
