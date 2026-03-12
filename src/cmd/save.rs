use crate::core::journal::Journal;
use anyhow::{Result, anyhow};
use crate::core::object_model::{Commit, Tree, TreeEntry};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::crypto::hash::hash_data;
use serde_json::json;
use std::fs;
use std::io::Read;
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
                let mut content = Vec::new();
                std::fs::File::open(&path)?.read_to_end(&mut content)?;

                let file_size = content.len();
                let threshold = 10 * 1024 * 1024; // 10MB

                let (hash, is_chunked, chunks) = if file_size > threshold {
                    let chunk_results = crate::storage::chunker::Chunker::chunk_data(&content);
                    let mut chunk_hashes = Vec::new();
                    for (c_hash, c_data) in chunk_results {
                        let compressed = crate::storage::compression::compress(&c_data)?;
                        store.put(c_hash.as_bytes(), &compressed)?;
                        chunk_hashes.push(c_hash);
                    }
                    // Ana hash tüm dosyanın hash'i olsun
                    (hash_data(&content), true, Some(chunk_hashes))
                } else {
                    let compressed = crate::storage::compression::compress(&content)?;
                    let h = hash_data(&content);
                    store.put(h.as_bytes(), &compressed)?;
                    (h, false, None)
                };

                entries.push(TreeEntry {
                    name: entry.path().display().to_string(),
                    hash,
                    is_dir: false,
                    is_chunked,
                    chunks,
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
        let head_content = fs::read_to_string(&head_path).ok();

        let parent_id = if let Some(ref content) = head_content {
            if content.starts_with("ref: ") {
                let ref_path = content.trim_start_matches("ref: ").trim();
                fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path)).ok()
            } else {
                Some(content.trim().to_string())
            }
        } else {
            None
        };

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

        // HEAD veya dalı güncelle
        if let Some(ref content) = head_content {
            if content.starts_with("ref: ") {
                let ref_path = content.trim_start_matches("ref: ").trim();
                fs::write(format!("{}/{}", ctx.repo_path, ref_path), &commit_hash)?;
            } else {
                // Detached HEAD durumundan main dalına otomatik geçiş
                let main_branch_path = format!("{}/refs/heads/main", ctx.repo_path);
                fs::create_dir_all(format!("{}/refs/heads", ctx.repo_path))?;
                fs::write(main_branch_path, &commit_hash)?;
                fs::write(&head_path, "ref: refs/heads/main")?;
            }
        } else {
            // Başlangıçta main dalı oluştur ve HEAD'i ona bağla
            let main_branch_path = format!("{}/refs/heads/main", ctx.repo_path);
            fs::create_dir_all(format!("{}/refs/heads", ctx.repo_path))?;
            fs::write(main_branch_path, &commit_hash)?;
            fs::write(&head_path, "ref: refs/heads/main")?;
        }

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
