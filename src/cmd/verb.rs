use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use crate::plugins::get_something_dir;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct VerbVerb;

impl VerbVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for VerbVerb {
    fn name(&self) -> &str {
        "verb"
    }

    fn help(&self) -> &str {
        "Komut eklentilerini yönetir: add, list, remove"
    }

    fn run(&self, _ctx: &ThingContext, args: &[String]) -> Result<()> {
        let verb_dir = get_something_dir().join("verbs");
        fs::create_dir_all(&verb_dir)?;

        if args.is_empty() || args[0] == "list" {
            println!("Yüklü komut eklentileri (tüm kaynaklar):");
            let paths = crate::plugins::get_plugin_search_paths("verbs");
            let mut count = 0;

            for dir_path in paths {
                if !dir_path.exists() { continue; }
                if let Ok(entries) = fs::read_dir(&dir_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("thing") {
                            let name = path.file_stem().unwrap().to_string_lossy();
                            println!("  - {: <10} ({})", name, path.display());
                            count += 1;
                        }
                    }
                }
            }
            if count == 0 {
                println!("  (Henüz eklenti yüklü değil)");
            }
            return Ok(());
        }

        match args[0].as_str() {
            "add" => {
                if args.len() < 2 {
                    return Err(anyhow!("Kullanım: hey verb add <dosya_yolu.thing>"));
                }
                let src_path = Path::new(&args[1]);
                if !src_path.exists() {
                    return Err(anyhow!("Dosya bulunamadı: {}", args[1]));
                }
                let file_name = src_path.file_name().ok_or_else(|| anyhow!("Geçersiz dosya"))?;
                
                // PROTECTED kontrolü
                let verb_name = src_path.file_stem().unwrap().to_string_lossy();
                if crate::plugins::verb_registry::is_protected(&verb_name) {
                    return Err(anyhow!("'{}' korumalı bir komuttur ve eklenti olarak eklenemez.", verb_name));
                }

                let dest_path = verb_dir.join(file_name);
                fs::copy(src_path, &dest_path)?;
                println!("Komut eklentisi başarıyla eklendi: {} (Sistemde artık 'hey {}' olarak kullanılabilir)", dest_path.display(), verb_name);
            }
            "remove" => {
                if args.len() < 2 {
                    return Err(anyhow!("Kullanım: hey verb remove <isim>"));
                }
                let name = &args[1];
                let target_path = verb_dir.join(format!("{}.thing", name));
                if !target_path.exists() {
                    return Err(anyhow!("Eklenti bulunamadı: {}", name));
                }
                fs::remove_file(&target_path)?;
                println!("Eklenti kaldırıldı: {}", name);
            }
            _ => {
                println!("Bilinmeyen alt komut: {}", args[0]);
                println!("Kullanım: hey verb [list | add | remove]");
            }
        }

        Ok(())
    }
}
