use crate::core::config::Config;
use crate::storage::kv_store::KvStore;
use anyhow::Result;
use std::path::Path;

pub struct ThingContext {
    pub repo_path: String,
    pub store: Option<KvStore>,
    pub config: Option<Config>,
}

impl ThingContext {
    pub fn load() -> Result<Self> {
        let repo_path = ".something".to_string();
        let config_path = ".configthing";

        let store = if Path::new(&repo_path).exists() {
            Some(KvStore::open(format!("{}/db", repo_path))?)
        } else {
            None
        };

        let config = if Path::new(config_path).exists() {
            Some(Config::load(config_path)?)
        } else {
            None
        };

        Ok(Self {
            repo_path,
            store,
            config,
        })
    }

    /// Nesneyi önce yerel depoda arar, yoksa ayarlı remote'dan çeker (Lazy Load / VFS)
    pub fn get_object(&self, hash: &str) -> Result<Vec<u8>> {
        let store = self.store.as_ref().ok_or_else(|| anyhow::anyhow!("Depo başlatılmamış."))?;
        
        // 1. Yerel kontrol
        if let Some(data) = store.get(hash.as_bytes())? {
            return Ok(data);
        }

        // 2. Remote kontrol (On-demand fetch)
        if let Some(ref config) = self.config {
            if let Some(ref remote_url) = config.somewhere.remote {
                println!("  [VFS] {} yerelde yok, remote'dan çekiliyor...", hash);
                
                let api_url = if remote_url.ends_with('/') {
                    format!("{}api/object/{}", remote_url, hash)
                } else {
                    format!("{}/api/object/{}", remote_url, hash)
                };

                let resp = reqwest::blocking::get(api_url)?;
                if resp.status().is_success() {
                    let data = resp.bytes()?.to_vec();
                    store.put(hash.as_bytes(), &data)?;
                    return Ok(data);
                }
            }
        }

        Err(anyhow::anyhow!("Nesne bulunamadı (Yerel ve Remote): {}", hash))
    }

    /// Delta zincirlerini çözerek objenin tam halini döndürür.
    pub fn get_reconstructed_blob(&self, hash: &str, entry_type: &crate::core::object_model::EntryType) -> Result<Vec<u8>> {
        use crate::core::object_model::EntryType;
        match entry_type {
            EntryType::Blob => {
                let compressed = self.get_object(hash)?;
                crate::storage::compression::decompress(&compressed)
            }
            EntryType::Delta => {
                let compressed_delta = self.get_object(hash)?;
                let delta_bin = crate::storage::compression::decompress(&compressed_delta)?;
                let delta_obj: crate::core::object_model::DeltaObject = bincode::deserialize(&delta_bin)?;
                
                let base_data = self.get_reconstructed_blob(&delta_obj.base_hash, &delta_obj.base_type)?;
                crate::storage::delta::DeltaEngine::apply_delta(&base_data, &delta_obj.patch)
            }
            EntryType::Tree => Err(anyhow::anyhow!("Tree bir blob olarak okunamaz.")),
        }
    }
}

pub trait VerbPlugin {
    fn name(&self) -> &str;
    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()>;
    fn help(&self) -> &str;
}
