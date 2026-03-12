use crate::core::object_model::{Commit, Tree};
use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct ShiftVerb;

impl ShiftVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for ShiftVerb {
    fn name(&self) -> &str {
        "shift"
    }

    fn aliases(&self) -> &[&str] {
        &[]
    }

    fn help(&self) -> &str {
        "Dallar veya commitler arasında geçiş yapar"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let target = args
            .first()
            .ok_or_else(|| anyhow!("Lütfen geçiş yapılacak dalı veya commit hash'ini belirtin."))?;
        let store = ctx
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış."))?;

        let branch_ref_path = format!("{}/refs/heads/{}", ctx.repo_path, target);
        let (commit_hash, is_branch) = if Path::new(&branch_ref_path).exists() {
            (
                fs::read_to_string(&branch_ref_path)?.trim().to_string(),
                true,
            )
        } else {
            (target.clone(), false)
        };

        // Commit'i doğrula
        let commit_data = store
            .get(commit_hash.as_bytes())?
            .ok_or_else(|| anyhow!("Commit bulunamadı: {}", commit_hash))?;
        let commit: Commit = serde_json::from_slice(&commit_data)?;

        // Tree'yi al ve dosyaları geri yükle (Basit çalışma dizini güncelleme)
        let tree_data = store
            .get(commit.tree_hash.as_bytes())?
            .ok_or_else(|| anyhow!("Ağaç bulunamadı: {}", commit.tree_hash))?;
        let tree: Tree = serde_json::from_slice(&tree_data)?;

        // TODO: Silinmesi gereken dosyaları temizle (Faz 2 gelişmiş checkout)
        for entry in tree.entries {
            if !entry.is_dir {
                let content = if entry.is_chunked {
                    let mut data = Vec::new();
                    if let Some(chunks) = entry.chunks {
                        for chunk_hash in chunks {
                            let compressed_chunk = store
                                .get(chunk_hash.as_bytes())?
                                .ok_or_else(|| anyhow!("Chunk bulunamadı: {}", chunk_hash))?;
                            let decompressed =
                                crate::storage::compression::decompress(&compressed_chunk)?;
                            data.extend(decompressed);
                        }
                    }
                    data
                } else {
                    let compressed_blob = store
                        .get(entry.hash.as_bytes())?
                        .ok_or_else(|| anyhow!("Dosya içeriği bulunamadı: {}", entry.hash))?;
                    crate::storage::compression::decompress(&compressed_blob)?
                };

                let path = Path::new(&entry.name);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, content)?;
            }
        }

        // HEAD'i güncelle
        let head_content = if is_branch {
            format!("ref: refs/heads/{}", target)
        } else {
            commit_hash.clone()
        };
        fs::write(format!("{}/HEAD", ctx.repo_path), head_content)?;

        println!("'{}' konumuna geçildi (Commit: {}).", target, commit_hash);
        Ok(())
    }
}
