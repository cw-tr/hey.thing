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
        if args.is_empty() {
            return Err(anyhow!("Hedef URL belirtilmedi. Örn: hey get http://somewhere.cw.tr"));
        }

        let url = &args[0];
        if !url.starts_with("http") {
            return Err(anyhow!("Local-to-Local get (pull) henüz desteklenmiyor. HTTP kullanın."));
        }

        println!("{} sunucusuna bağlanılıyor...", url);

        let repo_name = "test_project"; // Şimdilik hardcoded HTTP proxy prototype için

        let mut local_head = None;
        if let Some(_store) = &ctx.store {
            let head_path = std::path::Path::new(&ctx.repo_path).join("HEAD");
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
        } else {
             return Err(anyhow!("Önce 'hey init' çalıştırın, ardından 'hey get <url>' ile çekin."));
        }

        #[derive(serde::Serialize)]
        struct PullRequest<'a> {
            repo_name: &'a str,
            local_head: Option<String>,
        }

        let req_data = PullRequest {
            repo_name,
            local_head: local_head.clone(),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;

        let pull_url = format!("{}/api/pull", url.trim_end_matches('/'));
        
        println!("Sunucudan güncellemeler isteniyor...");
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
            println!("Sunucudan {} commit, {} tree, {} blob indirildi.", 
                delta.commits.len(), delta.trees.len(), delta.blobs.len());
            
            let store = ctx.store.as_ref().unwrap();

            for (hash, data) in delta.blobs {
                store.put(hash.as_bytes(), &data)?;
            }

            for (hash, data) in delta.trees {
                store.put(hash.as_bytes(), &data)?;
            }

            for (hash, data) in delta.commits {
                store.put(hash.as_bytes(), &data)?;
            }

            // HEAD güncelleme
            let head_path = std::path::Path::new(&ctx.repo_path).join("HEAD");
            let refs_dir = std::path::Path::new(&ctx.repo_path).join("refs").join("heads");
            std::fs::create_dir_all(&refs_dir)?;
            
            let main_ref = refs_dir.join("main");
            std::fs::write(&main_ref, &resp_data.remote_head)?;
            std::fs::write(&head_path, "ref: refs/heads/main")?;

            println!("Senkronizasyon başarılı, hedef commit: {}", resp_data.remote_head);
            println!("Öneri: İndirilen dosyaları (yeni state'i) çalışma alanına almak için `hey shift main` komutunu çalıştırın.");

        } else {
            println!("{}", resp_data.message);
        }

        Ok(())
    }
}
