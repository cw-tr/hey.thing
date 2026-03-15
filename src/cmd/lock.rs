use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use crate::crypto::auth::KeyManager;

pub struct LockVerb;

#[derive(Serialize)]
struct LockRequest {
    public_key: String,
    signature: String,
    path: String,
    owner_name: String,
}

#[derive(Deserialize)]
struct GenericResponse {
    success: bool,
    message: String,
}

impl VerbPlugin for LockVerb {
    fn name(&self) -> &str { "lock" }
    fn help(&self) -> &str { "Bir dosyayı Hub üzerinde kilitler (Binary Lock)" }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(anyhow!("Kullanım: hey lock <dosya_yolu>"));
        }
        let file_path = &args[0];

        // 1. Remote URL'yi al
        let remote_url = ctx.config.as_ref()
            .and_then(|c| c.somewhere.remote.as_ref())
            .ok_or_else(|| anyhow!("Hub (somewhere.remote) yapılandırılmamış."))?;

        // 2. İmza hazırla
        let signing_key = KeyManager::get_or_create_key()?;
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());
        let owner_name = ctx.config.as_ref()
            .map(|c| c.user.name.clone())
            .unwrap_or_else(|| "Anonymous".to_string());
        
        let payload = format!("lock:{}", file_path);
        let signature = KeyManager::sign(payload.as_bytes())?;

        // 3. İstek gönder
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/api/lock", remote_url);
        
        let res: GenericResponse = client.post(&url)
            .json(&LockRequest {
                public_key,
                signature,
                path: file_path.clone(),
                owner_name,
            })
            .send()?
            .json()?;

        if res.success {
            println!("  [+] {}", res.message);
        } else {
            println!("  [!] Başarısız: {}", res.message);
        }

        Ok(())
    }
}

pub struct UnlockVerb;

impl VerbPlugin for UnlockVerb {
    fn name(&self) -> &str { "unlock" }
    fn help(&self) -> &str { "Hub üzerindeki dosya kilidini kaldırır" }
    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Err(anyhow!("Kullanım: hey unlock <dosya_yolu>"));
        }
        let file_path = &args[0];

        let remote_url = ctx.config.as_ref()
            .and_then(|c| c.somewhere.remote.as_ref())
            .ok_or_else(|| anyhow!("Hub (somewhere.remote) yapılandırılmamış."))?;

        let signing_key = KeyManager::get_or_create_key()?;
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());
        
        let payload = format!("unlock:{}", file_path);
        let signature = KeyManager::sign(payload.as_bytes())?;

        let client = reqwest::blocking::Client::new();
        let url = format!("{}/api/unlock", remote_url);
        
        let res: GenericResponse = client.post(&url)
            .json(&LockRequest {
                public_key,
                signature,
                path: file_path.clone(),
                owner_name: String::new(), // Unlock için isim gerekmez
            })
            .send()?
            .json()?;

        if res.success {
            println!("  [+] {}", res.message);
        } else {
            println!("  [!] Başarısız: {}", res.message);
        }

        Ok(())
    }
}
