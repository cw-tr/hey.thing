use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::core::object_model::{Commit, Tree, EntryType};
use std::fs;
use std::path::Path;

use dashmap::DashSet;
use rayon::prelude::*;

pub struct SweepVerb;

impl SweepVerb {
    fn find_reachable(&self, ctx: &ThingContext, reachable: &DashSet<String>, hash: &str, obj_type: &str) -> Result<()> {
        if reachable.contains(hash) { return Ok(()); }
        reachable.insert(hash.to_string());

        let data = ctx.get_object(hash)?;
        let decompressed = crate::storage::compression::decompress(&data)?;

        match obj_type {
            "commit" => {
                let commit: Commit = bincode::deserialize(&decompressed)?;
                self.find_reachable(ctx, reachable, &commit.tree_hash, "tree")?;
                if let Some(parent) = commit.parent_id {
                    self.find_reachable(ctx, reachable, &parent, "commit")?;
                }
            }
            "tree" => {
                let tree: Tree = bincode::deserialize(&decompressed)?;
                tree.entries.par_iter().for_each(|entry| {
                    match entry.entry_type {
                        EntryType::Blob | EntryType::Delta => {
                            if entry.is_chunked {
                                if let Some(chunks) = &entry.chunks {
                                    for c_hash in chunks {
                                        reachable.insert(c_hash.clone());
                                    }
                                }
                            } else {
                                reachable.insert(entry.hash.clone());
                            }
                        }
                        EntryType::Tree => {
                            let _ = self.find_reachable(ctx, reachable, &entry.hash, "tree");
                        }
                    }
                });
            }
            _ => {}
        }
        Ok(())
    }
}

impl VerbPlugin for SweepVerb {
    fn name(&self) -> &str {
        "sweep"
    }

    fn help(&self) -> &str {
        "Kullanılmayan (yetim) nesneleri temizleyerek depoyu süpürür (GC)"
    }

    fn run(&self, ctx: &ThingContext, _args: &[String]) -> Result<()> {
        let store = ctx.store.as_ref().ok_or_else(|| anyhow!("Repo başlatılmamış."))?;
        println!("🧹 Depo süpürülüyor (Sweep)...");

        // Sistem kilitlenmesini önlemek için en az 1 çekirdeği boşta bırak
        let threads = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(1);
        
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|e| anyhow!("Thread pool oluşturulamadı: {}", e))?;

        let reachable = DashSet::new();
        
        pool.install(|| {
            // 1. Tüm referansları bul
            let refs_dir = Path::new(&ctx.repo_path).join("refs").join("heads");
            if refs_dir.exists() {
                if let Ok(entries) = fs::read_dir(refs_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if let Ok(hash) = fs::read_to_string(entry.path()) {
                                let hash = hash.trim().to_string();
                                if !hash.is_empty() {
                                    let _ = self.find_reachable(ctx, &reachable, &hash, "commit");
                                }
                            }
                        }
                    }
                }
            }
            
            let tags_dir = Path::new(&ctx.repo_path).join("refs").join("tags");
            if tags_dir.exists() {
                if let Ok(entries) = fs::read_dir(tags_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if let Ok(hash) = fs::read_to_string(entry.path()) {
                                let hash = hash.trim().to_string();
                                if !hash.is_empty() {
                                    let _ = self.find_reachable(ctx, &reachable, &hash, "commit");
                                }
                            }
                        }
                    }
                }
            }

            // 2. HEAD'i ekle
            let head_path = Path::new(&ctx.repo_path).join("HEAD");
            if head_path.exists() {
                if let Ok(content) = fs::read_to_string(head_path) {
                    let content = content.trim().to_string();
                    if !content.starts_with("ref: ") && !content.is_empty() {
                        let _ = self.find_reachable(ctx, &reachable, &content, "commit");
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        })?;

        println!("  [+] {} aktif nesne bulundu. ({} çekirdek kullanılıyor)", reachable.len(), threads);

        // 3. Süpür (Delete unreferenced)
        let mut deleted = 0;
        let mut total = 0;
        
        // Sled DB üzerinden silme işlemi için direkt erişim gerekiyor.
        // KvStore'a bir silme metodu eklemeliyiz.
        
        let all_keys: Vec<Vec<u8>> = store.iter().map(|(k, _)| k).collect();
        for k in all_keys {
            total += 1;
            let key_str = String::from_utf8_lossy(&k);
            if key_str.len() == 64 && !reachable.contains(key_str.as_ref()) {
                // Sil!
                if store.remove(&k).is_ok() {
                    deleted += 1;
                }
            }
        }

        println!("  [+] {} yetim nesne temizlendi.", deleted);
        println!("✅ Sweep tamamlandı. {total} nesneden {deleted} tanesi havaya uçtu.");

        Ok(())
    }
}
