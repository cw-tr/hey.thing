use crate::core::object_model::{Commit, Tree, EntryType};
use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;
use serde::Deserialize;

pub struct ShowVerb;

impl ShowVerb {
    pub fn new() -> Self {
        Self
    }

    fn list_tree(&self, ctx: &ThingContext, tree_hash: &str, filter_path: Option<&str>) -> Result<()> {
        let tree_data = ctx.get_object(tree_hash)?;
        let decompressed = crate::storage::compression::decompress(&tree_data)?;
        let tree: Tree = bincode::deserialize(&decompressed)?;

        if let Some(target) = filter_path {
            if target.is_empty() || target == "." {
                self.print_entries(&tree);
                return Ok(());
            }

            let parts: Vec<&str> = target.splitn(2, '/').collect();
            let first = parts[0];
            let rest = parts.get(1).cloned();

            for entry in tree.entries {
                if entry.name == first {
                    if entry.entry_type == EntryType::Tree {
                        return self.list_tree(ctx, &entry.hash, rest);
                    } else if rest.is_none() {
                        println!("- {} (Blob, {})", entry.name, entry.hash);
                        return Ok(());
                    }
                }
            }
            return Err(anyhow!("Yol bulunamadı: {}", target));
        } else {
            self.print_entries(&tree);
        }
        Ok(())
    }

    fn print_entries(&self, tree: &Tree) {
        println!("\nİçerik:");
        for entry in &tree.entries {
            let type_str = match entry.entry_type {
                EntryType::Blob => "📄",
                EntryType::Tree => "📁",
                EntryType::Delta => &format!("⚡ [D{}]", entry.delta_depth),
            };
            println!("  {}  {:<20}  [#{:o}]  {}", type_str, entry.name, entry.mode, entry.hash);
        }
    }
}

impl VerbPlugin for ShowVerb {
    fn name(&self) -> &str {
        "show"
    }

    fn help(&self) -> &str {
        "Proje durumunu ve ağaç yapısını gösterir. Kullanım: hey show [yol]"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let _store = ctx
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

        let commit_data = ctx.get_object(&head_hash)?;
        let decompressed = crate::storage::compression::decompress(&commit_data)?;
        let commit: Commit = bincode::deserialize(&decompressed)?;

        if let Some(bn) = branch_name {
            println!("Dal: {}", bn);
        }
        println!("Son Commit: {}", head_hash);
        println!("Yazar: {}", commit.author);
        println!("Mesaj: {}", commit.message);
        println!("Ağaç Kök Hash: {}", commit.tree_hash);

        // Belirli bir yolu listele
        let filter_path = args.first().map(|s| s.as_str());
        self.list_tree(ctx, &commit.tree_hash, filter_path)?;

        // --- Aktif Kilitleri Göster ---
        if let Some(remote_url) = ctx.config.as_ref().and_then(|c| c.somewhere.remote.as_ref()) {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(1))
                .build()?;
            
            let url = format!("{}/api/locks", remote_url);
            if let Ok(res) = client.get(&url).send() {
                if let Ok(locks_res) = res.json::<LocksResponse>() {
                    if !locks_res.locks.is_empty() {
                        println!("\n🔒 Hub Üzerindeki Kilitler:");
                        for lock in locks_res.locks {
                            println!("  - {}: {} tarafından kilitlendi", lock.path, lock.owner_name);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct LockInfo {
    path: String,
    owner_name: String,
}

#[derive(Deserialize)]
struct LocksResponse {
    locks: Vec<LockInfo>,
}
