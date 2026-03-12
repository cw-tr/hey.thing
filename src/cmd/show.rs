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

        let head_content = fs::read_to_string(&head_path)?;
        let (head_hash, branch_name) = if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ").trim();
            let hash = fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path))?;
            (
                hash.trim().to_string(),
                Some(ref_path.replace("refs/heads/", "")),
            )
        } else {
            (head_content.trim().to_string(), None)
        };

        let commit_data = store
            .get(head_hash.as_bytes())?
            .ok_or_else(|| anyhow!("Commit veritabanında bulunamadı: {}", head_hash))?;

        let commit: Commit = serde_json::from_slice(&commit_data)?;

        if let Some(bn) = branch_name {
            println!("Dal: {}", bn);
        }
        println!("Son Commit: {}", head_hash);
        println!("Yazar: {}", commit.author);
        println!("Mesaj: {}", commit.message);
        println!("Ağaç Hash: {}", commit.tree_hash);

        Ok(())
    }
}

use std::path::Path;
