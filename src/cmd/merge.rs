use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::core::object_model::Commit;
use std::fs;
use std::path::Path;

pub struct MergeVerb;

impl MergeVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for MergeVerb {
    fn name(&self) -> &str {
        "merge"
    }

    fn help(&self) -> &str {
        "Belirtilen dalı veya commit'i mevcut dala birleştirir (3-way semantic merge)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(anyhow!("Kullanım: hey merge <dal_adı_veya_commit_hash>"));
        }

        let target = &args[0];
        let store = ctx.store.as_ref().ok_or_else(|| anyhow!("Repo başlatılmamış."))?;

        // 1. Mevcut HEAD'i al (Local)
        let head_content = fs::read_to_string(format!("{}/HEAD", ctx.repo_path))?;
        let local_head = if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ").trim();
            fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path))?.trim().to_string()
        } else {
            head_content.trim().to_string()
        };

        // 2. Hedef commit'i al (Remote/Target)
        let refs_dir = format!("{}/refs/heads", ctx.repo_path);
        let target_ref_path = format!("{}/{}", refs_dir, target);
        let remote_head = if Path::new(&target_ref_path).exists() {
            fs::read_to_string(&target_ref_path)?.trim().to_string()
        } else {
            target.clone()
        };

        if local_head == remote_head {
            println!("Zaten güncel. Merge edilecek bir şey yok.");
            return Ok(());
        }

        // 3. Common Ancestor bul
        let ancestor = crate::core::sync::find_common_ancestor(store, &local_head, &remote_head)?;
        let anc_hash = ancestor.ok_or_else(|| anyhow!("Ortak ata bulunamadı. Merge imkansız."))?;

        println!("Merge başlatılıyor...");
        println!("  Local Head:  {}", local_head);
        println!("  Target Head: {}", remote_head);
        println!("  Ancestor:    {}", anc_hash);

        if anc_hash == remote_head {
            println!("Hedef zaten mevcut tarihin bir parçası (Zaten güncel).");
            return Ok(());
        }

        let repo_root = Path::new(&ctx.repo_path).parent().unwrap();

        // Fast-forward kontrolü (Eğer biz ancestry'de isek FF yapabiliriz)
        if anc_hash == local_head {
            println!("Fast-forward yapılabiliyor. Shift komutu ile dala geçebilirsiniz veya manuel FF...");
            // Basitlik için FF'i de birleştirme gibi yapalım ya da direkt shift çağrısı önerelim.
            // Şimdilik 3-way merge her zaman çalışır.
        }

        // 4. Perform Merge
        crate::core::sync::perform_merge(
            store,
            repo_root,
            &local_head,
            &remote_head,
            &anc_hash
        )?;

        println!("\n[+] Merge işlemi başarıyla tamamlandı.");
        println!("[!] Lütfen değişiklikleri kontrol edin ve 'hey save' ile kalıcı hale getirin.");

        Ok(())
    }
}
