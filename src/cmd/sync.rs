use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::storage::kv_store::KvStore;
use crate::core::sync::{find_common_ancestor, compute_delta};
use std::fs;
use std::path::Path;

pub struct SyncVerb;

impl SyncVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for SyncVerb {
    fn name(&self) -> &str {
        "sync"
    }

    fn aliases(&self) -> &[&str] {
        &[]
    }

    fn help(&self) -> &str {
        "Uzak veya yerel depo ile senkronize olur (push/pull)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(anyhow!("Hedef dizin (veya URL) belirtilmedi. Örn: hey sync /tmp/repo_b"));
        }

        let target = &args[0];
        
        // Şimdilik Phase 3 (Local-to-Local Prototiplendirmesi)
        // Eğer target bir HTTP linki değilse yerel dizin olarak kabul et
        if target.starts_with("http") {
            return sync_via_http(ctx, target);
        }

        let target_repo = Path::new(target).join(".something");
        if !target_repo.exists() {
            return Err(anyhow!("Belirtilen hedef konum geçerli bir hey.thing reposu değil: {}", target));
        }

        let local_store = ctx.store.as_ref()
            .ok_or_else(|| anyhow!("Yerel repo başlatılmamış."))?;

        let remote_store = KvStore::open(target_repo.join("db"))
            .map_err(|_| anyhow!("Uzak deponun veritabanına ulaşılamadı. Başka bir işlem kilitliyor olabilir."))?;

        let local_head = read_head_hash(&ctx.repo_path)?;
        let remote_head = read_head_hash(target_repo.to_str().unwrap())?;

        // Fast-forward kontrolü (ben de ondan daha ileri miyim yoksa ayrıştık mı?)
        let ancestor = find_common_ancestor(local_store, &local_head, &remote_head)?;

        if ancestor.as_deref() == Some(&remote_head) || remote_head.is_empty() {
            // Local, remotan'un doğrudan soyundan geliyor (Fast-Forward push uygun)
            println!("Senkronizasyon (push) başlıyor...");
            
            // Delta paketini hazırla
            let delta = compute_delta(local_store, &local_head, ancestor.as_deref())?;
            println!("Aktarılacak paket boyutu hesaplandı: {} commit, {} tree, {} blob",
                delta.commits.len(), delta.trees.len(), delta.blobs.len()
            );

            // Paketleri karşıya yükle (KV'ye yaz)
            // Önce blob'lar (dosya içerikleri)
            for (hash, data) in delta.blobs {
                remote_store.put(hash.as_bytes(), &data)?;
            }

            // Sonra tree'ler (dizin yapıları)
            for (hash, data) in delta.trees {
                remote_store.put(hash.as_bytes(), &data)?;
            }

            // En son commit'ler
            for (hash, data) in delta.commits {
                remote_store.put(hash.as_bytes(), &data)?;
            }

            // Karşı tarafın HEAD referansını güncelle
            overwrite_remote_head(target_repo.to_str().unwrap(), &local_head)?;

            println!("Senkronizasyon tamamlandı. Hedef sürüm başarıyla ilerletildi.");
            
        } else if ancestor.as_deref() == Some(&local_head) {
            // Remote benden ileride (pull gerekiyor, biz geri kalmışız)
            println!("Uzak depo sizden daha güncel ('pull' işlemi henüz desteklenmiyor).");
        } else {
            // Çatallanma (conflict var)
            println!("ÇATIŞMA (Conflict): Her iki depo da farklı yönlere evrilmiş.");
            println!("Bunu çözmek için AST-Merge arayüzü gereklidir. (Faz 4)");
        }

        Ok(())
    }
}

// ─── Yardımcı Fonksiyonlar ──────────────────────────────────────────

fn read_head_hash(repo_path: &str) -> Result<String> {
    let head_path = format!("{}/HEAD", repo_path);
    if !Path::new(&head_path).exists() {
        return Ok(String::new());
    }
    
    let content = fs::read_to_string(&head_path)?.trim().to_string();
    if content.starts_with("ref: ") {
        let ref_path = content.trim_start_matches("ref: ").trim();
        let target = format!("{}/{}", repo_path, ref_path);
        if Path::new(&target).exists() {
            Ok(fs::read_to_string(target)?.trim().to_string())
        } else {
            Ok(String::new())
        }
    } else {
        Ok(content)
    }
}

fn overwrite_remote_head(repo_path: &str, new_hash: &str) -> Result<()> {
    let head_path = format!("{}/HEAD", repo_path);
    if !std::path::Path::new(&head_path).exists() {
        let refs_dir = format!("{}/refs/heads", repo_path);
        std::fs::create_dir_all(&refs_dir)?;
        std::fs::write(format!("{}/main", refs_dir), new_hash)?;
        std::fs::write(&head_path, "ref: refs/heads/main")?;
        return Ok(());
    }

    let content = std::fs::read_to_string(&head_path)?.trim().to_string();
    
    if content.starts_with("ref: ") {
        let ref_path = content.trim_start_matches("ref: ").trim();
        let target_path = format!("{}/{}", repo_path, ref_path);
        if let Some(parent) = std::path::Path::new(&target_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target_path, new_hash)?;
    } else {
        std::fs::write(head_path, new_hash)?;
    }
    
    Ok(())
}

fn sync_via_http(ctx: &ThingContext, url: &str) -> Result<()> {
    use std::time::Duration;
    use crate::core::sync::DeltaPackage;

    println!("{} sunucusuna bağlanılıyor...", url);
    
    let local_store = ctx.store.as_ref()
        .ok_or_else(|| anyhow!("Yerel repo başlatılmamış."))?;

    let local_head = read_head_hash(&ctx.repo_path)?;
    
    // Basit olması adına şimdilik always push everything
    // Fast-Forward vb. logic HTTP üzerinden daha sonra geliştirilecek.
    // (Sunucudan once remote_head alinmali vs)
    // Prototype amaclı tum deltayi root'tan alacağız.
    let delta = compute_delta(local_store, &local_head, None)?;

    #[derive(serde::Serialize)]
    struct SyncRequest<'a> {
        delta: DeltaPackage,
        local_head: &'a str,
    }

    let request_data = SyncRequest {
        delta,
        local_head: &local_head,
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let sync_url = format!("{}/api/sync", url.trim_end_matches('/'));
    
    println!("Deltalar hesaplandı, aktarım başlıyor...");
    let response = client.post(&sync_url)
        .json(&request_data)
        .send()?;

    if response.status().is_success() {
        println!("Senkronizasyon başarılı!");
        Ok(())
    } else {
        Err(anyhow!("Sunucu hatası döndü: {}", response.status()))
    }
}
