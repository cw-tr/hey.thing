use anyhow::{anyhow, Result};
use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use crate::core::sync::DeltaPackage;
use std::time::Duration;

pub struct GetVerb;

impl GetVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for GetVerb {
    fn name(&self) -> &str {
        "get"
    }

    fn aliases(&self) -> &[&str] {
        &["pull"]
    }

    fn help(&self) -> &str {
        "Uzak depodan güncellemeleri veya tüm repoyu çeker (clone/pull)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        use crate::crypto::auth::KeyManager;
        use crate::core::sync::{find_common_ancestor, apply_checkout, perform_merge};
        use crate::core::object_model::Commit;

        if args.is_empty() {
            return Err(anyhow!("Hedef URL belirtilmedi. Örn: hey get http://somewhere.cw.tr"));
        }

        let url = &args[0];
        if !url.starts_with("http") {
            return Err(anyhow!("Local-to-Local get (pull) henüz desteklenmiyor. HTTP kullanın."));
        }

        println!("🛸 {} sunucusuna bağlanılıyor...", url);

        let store = ctx.store.as_ref().ok_or_else(|| anyhow!("Önce 'hey init' çalıştırın."))?;
        
        // Mevcut HEAD'i al
        let head_path = std::path::Path::new(&ctx.repo_path).join("HEAD");
        let mut local_head = None;
        if let Ok(content) = std::fs::read_to_string(&head_path) {
            let content = content.trim().to_string();
            if content.starts_with("ref: ") {
                let ref_path = content.trim_start_matches("ref: ").trim();
                let target = std::path::Path::new(&ctx.repo_path).join(ref_path);
                if let Ok(hash) = std::fs::read_to_string(target) {
                    local_head = Some(hash.trim().to_string());
                }
            } else if !content.is_empty() {
                local_head = Some(content);
            }
        }

        // --- AUTH: İmzalı Pull İsteği Hazırla ---
        let signing_key = KeyManager::get_or_create_key()?;
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        
        let sign_payload = local_head.clone().unwrap_or_else(|| "clone".to_string());
        let signature = KeyManager::sign(sign_payload.as_bytes())?;

        #[derive(serde::Serialize)]
        struct PullRequest {
            public_key: String,
            signature: String,
            local_head: Option<String>,
        }

        let req_data = PullRequest {
            public_key: public_key_hex,
            signature,
            local_head: local_head.clone(),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;

        let pull_url = format!("{}/api/pull", url.trim_end_matches('/'));
        
        println!("📡 Sunucudan güncellemeler isteniyor...");
        let response = client.post(&pull_url).json(&req_data).send()?;

        if !response.status().is_success() {
            return Err(anyhow!("Sunucu hatası: {}", response.status()));
        }

        #[derive(serde::Deserialize)]
        struct PullResponse {
            success: bool,
            message: String,
            remote_head: String,
            delta: Option<DeltaPackage>,
        }

        let resp_data: PullResponse = response.json()?;

        if !resp_data.success {
            return Err(anyhow!("Pull başarısız: {}", resp_data.message));
        }

        if let Some(delta) = resp_data.delta {
            println!("✅ Sunucudan {} commit, {} tree, {} blob indirildi.", 
                delta.commits.len(), delta.trees.len(), delta.blobs.len());
            
            // 1. Verileri database'e yaz
            for (hash, data) in delta.blobs { store.put(hash.as_bytes(), &data)?; }
            for (hash, data) in delta.trees { store.put(hash.as_bytes(), &data)?; }
            for (hash, data) in delta.commits { store.put(hash.as_bytes(), &data)?; }

            let remote_head = &resp_data.remote_head;

            // 2. State Analizi (Fast-Forward mı? Merge mü?)
            if let Some(l_head) = local_head {
                let ancestor = find_common_ancestor(store, &l_head, remote_head)?;
                
                if ancestor.as_deref() == Some(&l_head) {
                    println!("🚀 Fast-Forward tespit edildi. Çalışma dizini güncelleniyor...");
                    let remote_commit: Commit = serde_json::from_slice(&store.get(remote_head.as_bytes())?.unwrap())?;
                    apply_checkout(store, &remote_commit.tree_hash, std::path::Path::new(&ctx.repo_path).parent().unwrap())?;
                } else if ancestor.as_deref() == Some(remote_head) {
                    println!("✨ Yerel depo zaten daha güncel.");
                } else if let Some(ancest) = ancestor {
                    println!("⚔️  Farklılaşma (Diverge) tespit edildi. 3-way merge başlatılıyor...");
                    perform_merge(store, std::path::Path::new(&ctx.repo_path).parent().unwrap(), &l_head, remote_head, &ancest)?;
                    println!("📢 Merge tamamlandı. Çatışmaları kontrol edip `hey save` yapmayı unutmayın.");
                } else {
                    println!("🌑 Ortak geçmiş bulunamadı. Temiz checkout yapılıyor...");
                    let remote_commit: Commit = serde_json::from_slice(&store.get(remote_head.as_bytes())?.unwrap())?;
                    apply_checkout(store, &remote_commit.tree_hash, std::path::Path::new(&ctx.repo_path).parent().unwrap())?;
                }
            } else {
                // İlk defa çekiliyor (Clone benzeri)
                println!("📦 İlk çekim yapılıyor. Dosyalar çıkarılıyor...");
                let remote_commit: Commit = serde_json::from_slice(&store.get(remote_head.as_bytes())?.unwrap())?;
                apply_checkout(store, &remote_commit.tree_hash, std::path::Path::new(&ctx.repo_path).parent().unwrap())?;
            }

            // 3. HEAD ve Ref güncelleme
            let refs_dir = std::path::Path::new(&ctx.repo_path).join("refs").join("heads");
            std::fs::create_dir_all(&refs_dir)?;
            std::fs::write(refs_dir.join("main"), remote_head)?;
            std::fs::write(head_path, "ref: refs/heads/main")?;

            println!("🏁 İşlem başarıyla tamamlandı. Mevcut commit: {}", remote_head);

        } else {
            println!("💡 {}", resp_data.message);
        }

        Ok(())
    }
}
