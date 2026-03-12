use crate::core::journal::Journal;
use crate::core::object_model::{Commit, Tree, TreeEntry};
use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use crate::crypto::hash::hash_data;
use anyhow::{Result, anyhow};
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

pub struct SaveVerb;

impl SaveVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for SaveVerb {
    fn name(&self) -> &str {
        "save"
    }

    fn aliases(&self) -> &[&str] {
        &["s", "commit"]
    }

    fn help(&self) -> &str {
        "Değişiklikleri kaydeder (git commit -am gibi)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let store = ctx
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış. 'hey init' çalıştırın."))?;

        let message = args
            .first()
            .cloned()
            .unwrap_or_else(|| "no message".to_string());

        let mut entries = Vec::new();

        // Çalışma dizinini tara
        for entry in WalkDir::new(".")
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".something" && name != "target" && name != ".git"
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                let content = fs::read(path)?;
                let hash = hash_data(&content);

                // Blob'u kaydet
                store.put(hash.as_bytes(), &content)?;

                entries.push(TreeEntry {
                    name: entry.path().to_string_lossy().replace("./", ""),
                    hash,
                    is_dir: false,
                });
            }
        }

        // Tree nesnesini oluştur ve kaydet
        let tree = Tree { entries };
        let tree_json = serde_json::to_vec(&tree)?;
        let tree_hash = hash_data(&tree_json);
        store.put(tree_hash.as_bytes(), &tree_json)?;

        // Mevcut HEAD'i oku (parent için)
        let head_path = format!("{}/HEAD", ctx.repo_path);
        let parent_id = fs::read_to_string(&head_path).ok();

        // Commit nesnesini oluştur
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let author = ctx
            .config
            .as_ref()
            .map(|c| c.user.name.clone())
            .unwrap_or_else(|| "Anonymous".to_string());

        let commit = Commit {
            parent_id,
            tree_hash: tree_hash.clone(),
            author,
            timestamp,
            message: message.clone(),
        };

        let commit_json = serde_json::to_vec(&commit)?;
        let commit_hash = hash_data(&commit_json);
        store.put(commit_hash.as_bytes(), &commit_json)?;

        // HEAD'i güncelle
        fs::write(head_path, &commit_hash)?;

        // Journal kaydı
        Journal::log(
            "save",
            json!({
                "commit_hash": commit_hash,
                "message": message,
                "tree_hash": tree_hash
            }),
        )?;

        println!("Kaydedildi: {}", commit_hash);
        Ok(())
    }
}
