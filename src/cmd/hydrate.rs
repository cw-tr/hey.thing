use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::core::object_model::{Commit, Tree, EntryType};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct HydrateVerb;

impl HydrateVerb {
    pub fn new() -> Self {
        Self
    }

    fn find_in_tree(&self, ctx: &ThingContext, tree_hash: &str, target_parts: &[&str]) -> Result<(String, EntryType, u32, bool, Option<Vec<String>>)> {
        let tree_data = ctx.get_object(tree_hash)?;
        let decompressed = crate::storage::compression::decompress(&tree_data)?;
        let tree: Tree = bincode::deserialize(&decompressed)?;

        for entry in tree.entries {
            if entry.name == target_parts[0] {
                if target_parts.len() == 1 {
                    return Ok((entry.hash, entry.entry_type, entry.mode, entry.is_chunked, entry.chunks));
                } else if entry.entry_type == EntryType::Tree {
                    return self.find_in_tree(ctx, &entry.hash, &target_parts[1..]);
                }
            }
        }
        Err(anyhow!("Dosya ağaçta bulunamadı: {}", target_parts.join("/")))
    }
}

impl VerbPlugin for HydrateVerb {
    fn name(&self) -> &str {
        "hydrate"
    }

    fn help(&self) -> &str {
        "Ghost (0-byte) dosyayı gerçek içeriğiyle doldurur (Lazy Load Hydration)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.is_empty() {
             println!("Kullanım: hey hydrate <dosya-yolu>");
             return Ok(());
        }

        let target_path_str = &args[0];
        let target_path = Path::new(target_path_str);

        // 1. Mevcut HEAD'i bul
        let head_path = format!("{}/HEAD", ctx.repo_path);
        if !Path::new(&head_path).exists() {
            return Err(anyhow!("HEAD bulunamadı."));
        }

        let head_content = fs::read_to_string(&head_path)?;
        let head_hash = if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ").trim();
            fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path))?.trim().to_string()
        } else {
            head_content.trim().to_string()
        };

        // 2. Commit'i oku
        let commit_data = ctx.get_object(&head_hash)?;
        let commit_dec = crate::storage::compression::decompress(&commit_data)?;
        let commit: Commit = bincode::deserialize(&commit_dec)?;

        // 3. Dosyayı Tree içinde ara
        let normalized_path = target_path_str.replace("\\", "/");
        let parts: Vec<&str> = normalized_path.split('/').filter(|s| !s.is_empty()).collect();
        
        println!("💧 '{}' içeriği çekiliyor...", target_path_str);
        
        let (hash, entry_type, mode, is_chunked, chunks) = self.find_in_tree(ctx, &commit.tree_hash, &parts)?;

        if entry_type == EntryType::Tree {
            return Err(anyhow!("Bir klasörü hydrate edemezsiniz, lütfen içindeki bir dosyayı seçin."));
        }

        // 4. İçeriği rekonstrükt et
        let content = if is_chunked {
            let mut data = Vec::new();
            if let Some(c_hashes) = chunks {
                for c_hash in c_hashes {
                    let compressed_chunk = ctx.get_object(&c_hash)?;
                    let decompressed = crate::storage::compression::decompress(&compressed_chunk)?;
                    data.extend(decompressed);
                }
            }
            data
        } else {
            ctx.get_reconstructed_blob(&hash, &entry_type)?
        };

        // 5. Dosyaya yaz
        fs::write(target_path, content)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(target_path, fs::Permissions::from_mode(mode))?;
        }

        println!("✨ '{}' başarıyla dolduruldu. ({} bytes)", target_path_str, fs::metadata(target_path)?.len());

        Ok(())
    }
}
