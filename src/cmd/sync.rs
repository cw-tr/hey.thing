use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::storage::kv_store::KvStore;
use crate::core::sync::{compute_delta, apply_checkout};
use crate::core::object_model::Commit;
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
        "Uzak veya yerel depo ile senkronize olur. Argümansız kullanımda configdeki tüm hedeflere (backup, remote) sırayla aktarır."
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let config = crate::core::config::Config::load(".configthing").ok();
        
        let mut targets = Vec::new();

        if args.is_empty() {
            // Argüman yoksa zincirleme (Chain) mod: backup -> remote
            if let Some(cfg) = &config {
                if let Some(b) = &cfg.somewhere.backup {
                    targets.push(b.clone());
                }
                if let Some(r) = &cfg.somewhere.remote {
                    targets.push(r.clone());
                }
            }
            
            if targets.is_empty() {
                 return Err(anyhow!("Senkronizasyon için hedef bulunamadı. Lütfen .configthing dosyasını veya manuel URL belirtin."));
            }
        } else {
            // Argüman varsa ya 'backup', ya 'remote' ya da direkt yol/URL'dir
            let arg = &args[0];
            if let Some(cfg) = &config {
                if arg == "backup" {
                    if let Some(b) = &cfg.somewhere.backup { targets.push(b.clone()); }
                } else if arg == "remote" {
                    if let Some(r) = &cfg.somewhere.remote { targets.push(r.clone()); }
                }
            }
            
            if targets.is_empty() {
                targets.push(arg.clone()); // Doğrudan URL veya Yol kabul et
            }
        }

        for target in targets {
            println!("--- Hedef senkronize ediliyor: {} ---", target);
            if let Err(e) = sync_to_target(ctx, &target) {
                println!("HATA: {} hedefine senkronizasyon başarısız: {}", target, e);
            }
        }

        Ok(())
    }
}

fn sync_to_target(ctx: &ThingContext, target: &str) -> Result<()> {
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

    let ancestor = crate::core::sync::find_common_ancestor_cross(local_store, &local_head, &remote_store, &remote_head)?;

    if ancestor.as_deref() == Some(&remote_head) || remote_head.is_empty() {
        println!("Senkronizasyon (push) başlıyor...");
        let delta = compute_delta(local_store, &local_head, ancestor.as_deref())?;
        
        let total_blobs = delta.blobs.len();
        let total_trees = delta.trees.len();
        let total_commits = delta.commits.len();
        println!("  [+] {} commit, {} ağaç, {} blob aktarılıyor...", total_commits, total_trees, total_blobs);
        println!("  [+] Aktarılacak paket boyutu: ~{} KB", (total_blobs + total_trees + total_commits) * 4); // Yaklaşık

        for (hash, data) in delta.blobs { remote_store.put(hash.as_bytes(), &data)?; }
        for (hash, data) in delta.trees { remote_store.put(hash.as_bytes(), &data)?; }
        for (hash, data) in delta.commits { remote_store.put(hash.as_bytes(), &data)?; }

        overwrite_remote_head(target_repo.to_str().unwrap(), &local_head)?;
        println!("Senkronizasyon tamamlandı.");
        
    } else if ancestor.as_deref() == Some(&local_head) {
        // Karşı taraf bizden daha önde → local repo pull ile güncellenebilir
        println!("UYARI: Uzak (backup) depo sizden daha güncel.");
        println!("Yerel çalışma dizini fast-forward ile güncelleniyor...");
        let delta = compute_delta(&remote_store, &remote_head, ancestor.as_deref())?;
        for (hash, data) in delta.blobs { local_store.put(hash.as_bytes(), &data)?; }
        for (hash, data) in delta.trees { local_store.put(hash.as_bytes(), &data)?; }
        for (hash, data) in delta.commits { local_store.put(hash.as_bytes(), &data)?; }
        let remote_commit_data = local_store.get(remote_head.as_bytes())?
            .ok_or_else(|| anyhow!("Uzak commit objesi yerel depoda bulunamadı: {}", remote_head))?;
        let remote_commit: Commit = serde_json::from_slice(&remote_commit_data)?;
        let work_dir = Path::new(&ctx.repo_path).parent().unwrap();
        apply_checkout(local_store, &remote_commit.tree_hash, work_dir)?;
        overwrite_remote_head(&ctx.repo_path, &remote_head)?;
        println!("Yerel depo güncellendi: {}", remote_head);

    } else {
        if let Some(anc) = ancestor {
            println!("ÇATIŞMA: Her iki depo da farklı yönlere evrilmiş. 3-way merge deneniyor...");
            
            // Merge öncesi remote'daki verileri yerel store'a çekmeliyiz ki merge işlemi yapabilsin
            let delta = compute_delta(&remote_store, &remote_head, Some(&anc))?;
            for (hash, data) in delta.blobs { local_store.put(hash.as_bytes(), &data)?; }
            for (hash, data) in delta.trees { local_store.put(hash.as_bytes(), &data)?; }
            for (hash, data) in delta.commits { local_store.put(hash.as_bytes(), &data)?; }

            crate::core::sync::perform_merge(
                local_store,
                std::path::Path::new(&ctx.repo_path).parent().unwrap(),
                &local_head,
                &remote_head,
                &anc
            )?;
            println!("Merge işlemi tamamlandı. Lütfen kontrol edip save yapın.");
        } else {
            return Err(anyhow!("Ortak ata (common ancestor) bulunamadı. Develer ayrı yönlere gitmiş, otomatik merge imkansız."));
        }
    }

    Ok(())
}


// sync_via_http, read_head_hash, overwrite_remote_head fonksiyonları aynen korunmalı...
fn read_head_hash(repo_path: &str) -> Result<String> {
    let head_path = format!("{}/HEAD", repo_path);
    if !Path::new(&head_path).exists() { return Ok(String::new()); }
    let content = fs::read_to_string(&head_path)?.trim().to_string();
    if content.starts_with("ref: ") {
        let ref_path = content.trim_start_matches("ref: ").trim();
        let target = format!("{}/{}", repo_path, ref_path);
        if Path::new(&target).exists() { Ok(fs::read_to_string(target)?.trim().to_string()) } else { Ok(String::new()) }
    } else { Ok(content) }
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
        if let Some(parent) = std::path::Path::new(&target_path).parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(target_path, new_hash)?;
    } else { std::fs::write(head_path, new_hash)?; }
    Ok(())
}

fn sync_via_http(ctx: &ThingContext, url: &str) -> Result<()> {
    use std::time::Duration;
    use crate::core::sync::DeltaPackage;
    use crate::crypto::auth::KeyManager;

    let local_store = ctx.store.as_ref().ok_or_else(|| anyhow!("Yerel repo başlatılmamış."))?;
    let local_head = read_head_hash(&ctx.repo_path)?;
    let delta = compute_delta(local_store, &local_head, None)?;

    #[derive(serde::Serialize)]
    struct SyncRequest<'a> {
        public_key: String,
        signature: String,
        local_head: &'a str,
        delta: DeltaPackage,
    }

    let signing_key = KeyManager::get_or_create_key()?;
    let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
    let delta_json = serde_json::to_vec(&delta)?;
    let delta_hash = crate::crypto::hash::hash_data(&delta_json);
    let sign_payload = format!("{}:{}", local_head, delta_hash);
    let signature = KeyManager::sign(sign_payload.as_bytes())?;

    let request_data = SyncRequest { public_key: public_key_hex, signature, local_head: &local_head, delta };
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(300)).build()?;
    let sync_url = format!("{}/api/sync", url.trim_end_matches('/'));
    
    let response = client.post(&sync_url).json(&request_data).send()?;
    if response.status().is_success() {
        println!("Senkronizasyon başarılı!");
        Ok(())
    } else {
        Err(anyhow!("Sunucu hatası döndü: {}", response.status()))
    }
}
