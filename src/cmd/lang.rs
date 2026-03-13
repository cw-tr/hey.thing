use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use crate::plugins::get_something_dir;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct LangVerb;

impl LangVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for LangVerb {
    fn name(&self) -> &str {
        "lang"
    }

    fn aliases(&self) -> &[&str] {
        &[]
    }

    fn help(&self) -> &str {
        "Dil eklentilerini (AST Merge) yönetir: add, list, remove"
    }

    fn run(&self, _ctx: &ThingContext, args: &[String]) -> Result<()> {
        let lang_dir = get_something_dir().join("langs");
        fs::create_dir_all(&lang_dir)?;

        if args.is_empty() || args[0] == "list" {
            println!("Yüklü dil eklentileri (tüm kaynaklar):");
            let paths = crate::plugins::get_plugin_search_paths("langs");
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
                    return Err(anyhow!("Kullanım: hey lang add <dosya_yolu.thing>"));
                }
                let src_path = Path::new(&args[1]);
                if !src_path.exists() {
                    return Err(anyhow!("Dosya bulunamadı: {}", args[1]));
                }
                let file_name = src_path.file_name().ok_or_else(|| anyhow!("Geçersiz dosya"))?;
                let dest_path = lang_dir.join(file_name);
                
                fs::copy(src_path, &dest_path)?;
                println!("Eklenti başarıyla eklendi: {}", dest_path.display());
            }
            "remove" => {
                if args.len() < 2 {
                    return Err(anyhow!("Kullanım: hey lang remove <isim>"));
                }
                let name = &args[1];
                let target_path = lang_dir.join(format!("{}.thing", name));
                if !target_path.exists() {
                    return Err(anyhow!("Eklenti bulunamadı: {}", name));
                }
                fs::remove_file(&target_path)?;
                println!("Eklenti kaldırıldı: {}", name);
            }
            _ => {
                println!("Bilinmeyen alt komut: {}", args[0]);
                println!("Kullanım: hey lang [list | add | remove]");
            }
        }

        Ok(())
    }
}
