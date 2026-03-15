use crate::core::object_model::Commit;
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

    fn help(&self) -> &str {
        "Dalları listeler, yeni yol(dal) açar veya dallar/commitler arasında geçiş yapar"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let refs_dir = format!("{}/refs/heads", ctx.repo_path);
        fs::create_dir_all(&refs_dir)?;

        if args.is_empty() {
            // Dalları listele
            let current_branch =
                fs::read_to_string(format!("{}/HEAD", ctx.repo_path)).unwrap_or_default();

            for entry in fs::read_dir(&refs_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let is_current = if current_branch.starts_with("ref: refs/heads/") {
                    current_branch.trim().ends_with(&name)
                } else {
                    false
                };

                if is_current {
                    println!("* {}", name);
                } else {
                    println!("   {}", name);
                }
            }
            return Ok(());
        }

        if args.len() >= 2 && args[0] == "new" {
            let branch_name = &args[1];
            let branch_file = format!("{}/{}", refs_dir, branch_name);

            if Path::new(&branch_file).exists() {
                return Err(anyhow!("'{}' adında bir dal/yol zaten mevcut.", branch_name));
            }

            // Mevcut HEAD commit'ini al
            let head_content = fs::read_to_string(format!("{}/HEAD", ctx.repo_path))?;
            let current_hash = if head_content.starts_with("ref: ") {
                let ref_path = head_content.trim_start_matches("ref: ").trim();
                fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path))?
            } else {
                head_content.trim().to_string()
            };

            // Yeni dalı oluştur
            fs::write(&branch_file, current_hash)?;

            // Yeni dala geç (shift)
            fs::write(
                format!("{}/HEAD", ctx.repo_path),
                format!("ref: refs/heads/{}", branch_name),
            )?;

            println!("'{}' yolu oluşturuldu ve geçiş yapıldı.", branch_name);
            return Ok(());
        }

        let is_lazy = args.contains(&"--lazy".to_string());
        let target = args.iter().find(|a| !a.starts_with("--")).ok_or_else(|| anyhow!("Hedef belirtilmedi."))?;

        let _store = ctx
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış."))?;

        let branch_ref_path = format!("{}/{}", refs_dir, target);
        let (commit_hash, is_branch) = if Path::new(&branch_ref_path).exists() {
            (
                fs::read_to_string(&branch_ref_path)?.trim().to_string(),
                true,
            )
        } else {
            (target.clone(), false)
        };

        // Commit'i doğrula
        let commit_data = ctx.get_object(&commit_hash)?;
        let decompressed = crate::storage::compression::decompress(&commit_data)?;
        let commit: Commit = bincode::deserialize(&decompressed)?;
 
        // Tree'yi al ve dosyaları geri yükle (Rekürsif Checkout)
        let work_dir = Path::new(&ctx.repo_path).parent().unwrap_or(Path::new("."));
        
        if is_lazy {
            println!("👻 Ghost Checkout (Lazy) başlatılıyor...");
            recursive_ghost_checkout(ctx, &commit.tree_hash, work_dir)?;
        } else {
            recursive_checkout(ctx, &commit.tree_hash, work_dir)?;
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

fn recursive_ghost_checkout(ctx: &ThingContext, tree_hash: &str, current_path: &Path) -> Result<()> {
    use crate::core::object_model::{Tree, EntryType};
    let tree_data = ctx.get_object(tree_hash)?;
    let decompressed = crate::storage::compression::decompress(&tree_data)?;
    let tree: Tree = bincode::deserialize(&decompressed)?;

    for entry in tree.entries {
        if entry.name == ".configthing" || entry.name == ".something" {
            continue;
        }
        let entry_path = current_path.join(&entry.name);
        match entry.entry_type {
            EntryType::Blob | EntryType::Delta => {
                if let Some(parent) = entry_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // Ghost file: 0 byte. Content will be hydrated later.
                std::fs::write(&entry_path, "")?;
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&entry_path, std::fs::Permissions::from_mode(entry.mode))?;
                }
            }
            EntryType::Tree => {
                fs::create_dir_all(&entry_path)?;
                recursive_ghost_checkout(ctx, &entry.hash, &entry_path)?;
            }
        }
    }
    Ok(())
}

fn recursive_checkout(ctx: &ThingContext, tree_hash: &str, current_path: &Path) -> Result<()> {
    use crate::core::object_model::{Tree, EntryType};
    let tree_data = ctx.get_object(tree_hash)?;
    let decompressed = crate::storage::compression::decompress(&tree_data)?;
    let tree: Tree = bincode::deserialize(&decompressed)?;

    for entry in tree.entries {
        if entry.name == ".configthing" || entry.name == ".something" {
            continue;
        }
        let entry_path = current_path.join(&entry.name);
        match entry.entry_type {
            EntryType::Blob | EntryType::Delta => {
                let content = if entry.is_chunked {
                    let mut data = Vec::new();
                    if let Some(chunks) = entry.chunks {
                        for chunk_hash in chunks {
                            let compressed_chunk = ctx.get_object(&chunk_hash)?;
                            let decompressed = crate::storage::compression::decompress(&compressed_chunk)?;
                            data.extend(decompressed);
                        }
                    }
                    data
                } else {
                    ctx.get_reconstructed_blob(&entry.hash, &entry.entry_type)?
                };

                if let Some(parent) = entry_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&entry_path, content)?;

                // File mode support
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&entry_path, std::fs::Permissions::from_mode(entry.mode))?;
                }
            }
            EntryType::Tree => {
                fs::create_dir_all(&entry_path)?;
                recursive_checkout(ctx, &entry.hash, &entry_path)?;
            }
        }
    }
    Ok(())
}
