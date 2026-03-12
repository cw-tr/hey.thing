use crate::core::object_model::Commit;
use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::{Result, anyhow};
use std::fs;

pub struct ShowVerb;

impl ShowVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for ShowVerb {
    fn name(&self) -> &str {
        "show"
    }

    fn aliases(&self) -> &[&str] {
        &["st", "status"]
    }

    fn help(&self) -> &str {
        "Proje durumunu ve son commit'i gösterir"
    }

    fn run(&self, ctx: &ThingContext, _args: &[String]) -> Result<()> {
        let store = ctx
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış."))?;
        let head_path = format!("{}/HEAD", ctx.repo_path);

        if !Path::new(&head_path).exists() {
            println!("Henüz bir commit atılmamış.");
            return Ok(());
        }

        let head_hash = fs::read_to_string(&head_path)?;
        let commit_data = store
            .get(head_hash.as_bytes())?
            .ok_or_else(|| anyhow!("HEAD commit'i veritabanında bulunamadı."))?;

        let commit: Commit = serde_json::from_slice(&commit_data)?;

        println!("Son Commit: {}", head_hash);
        println!("Yazar: {}", commit.author);
        println!("Mesaj: {}", commit.message);
        println!("Ağaç Hash: {}", commit.tree_hash);

        Ok(())
    }
}

use std::path::Path;
