use crate::core::journal::Journal;
use anyhow::{Result, anyhow};
use crate::core::object_model::{Commit, Tree};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::crypto::hash::hash_data;
use serde_json::json;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;
use std::collections::HashMap;
use crate::storage::delta::DeltaEngine;
use crate::core::object_model::DeltaObject;

pub struct SaveVerb;

impl SaveVerb {
    pub fn new() -> Self {
        Self
    }

    fn offline_cache(&self, ctx: &ThingContext) -> Result<()> {
        println!("🚀 Çevrimdışı önbelleğe alma (Offline Cache) başlatılıyor...");
        
        let head_path = format!("{}/HEAD", ctx.repo_path);
        if !Path::new(&head_path).exists() {
            return Err(anyhow!("Henüz bir commit yok, önbelleklenecek bir şey bulunamadı."));
        }

        let head_content = fs::read_to_string(&head_path)?;
        let head_hash = if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ").trim();
            fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path))?.trim().to_string()
        } else {
            head_content.trim().to_string()
        };

        // Rekürsif olarak tüm nesneleri çek
        self.fetch_recursive(ctx, &head_hash, "commit")?;

        println!("\n✨ Tüm nesneler yerel depoya kilitlendi. Artık çevrimdışı çalışabilirsiniz.");
        Ok(())
    }

    fn fetch_recursive(&self, ctx: &ThingContext, hash: &str, obj_type: &str) -> Result<()> {
        let data = ctx.get_object(hash)?;
        let decompressed = crate::storage::compression::decompress(&data)?;
        
        match obj_type {
            "commit" => {
                let commit: Commit = bincode::deserialize(&decompressed)?;
                self.fetch_recursive(ctx, &commit.tree_hash, "tree")?;
                if let Some(parent) = commit.parent_id {
                    self.fetch_recursive(ctx, &parent, "commit")?;
                }
            }
            "tree" => {
                let tree: Tree = bincode::deserialize(&decompressed)?;
                for entry in tree.entries {
                    match entry.entry_type {
                        crate::core::object_model::EntryType::Blob | crate::core::object_model::EntryType::Delta => {
                            if entry.is_chunked {
                                if let Some(chunks) = entry.chunks {
                                    for c_hash in chunks {
                                        ctx.get_object(&c_hash)?;
                                    }
                                }
                            } else {
                                let data = ctx.get_object(&entry.hash)?;
                                if entry.entry_type == crate::core::object_model::EntryType::Delta {
                                    // Delta ise base'ini de çek
                                    if let Ok(dec) = crate::storage::compression::decompress(&data) {
                                        if let Ok(delta_obj) = bincode::deserialize::<crate::core::object_model::DeltaObject>(&dec) {
                                            self.fetch_delta_base(ctx, &delta_obj)?;
                                        }
                                    }
                                }
                            }
                        }
                        crate::core::object_model::EntryType::Tree => {
                            self.fetch_recursive(ctx, &entry.hash, "tree")?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn fetch_delta_base(&self, ctx: &ThingContext, delta: &crate::core::object_model::DeltaObject) -> Result<()> {
        let data = ctx.get_object(&delta.base_hash)?;
        if delta.base_type == crate::core::object_model::EntryType::Delta {
             if let Ok(dec) = crate::storage::compression::decompress(&data) {
                if let Ok(sub_delta) = bincode::deserialize::<crate::core::object_model::DeltaObject>(&dec) {
                    self.fetch_delta_base(ctx, &sub_delta)?;
                }
            }
        }
        Ok(())
    }
}

impl VerbPlugin for SaveVerb {
    fn name(&self) -> &str {
        "save"
    }

    fn help(&self) -> &str {
        "Değişiklikleri kaydeder (git commit -am gibi)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.contains(&"--offline-cache".to_string()) {
           return self.offline_cache(ctx);
        }

        let store = ctx
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış. 'hey init' çalıştırın."))?;

        let message = args
            .first()
            .cloned()
            .unwrap_or_else(|| "no message".to_string());

        let mut hub_locks = Vec::new();
        if let Some(remote_url) = ctx.config.as_ref().and_then(|c| c.somewhere.remote.as_ref()) {
            if let Ok(client) = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(1))
                .build()
            {
                let url = format!("{}/api/locks", remote_url);
                if let Ok(res) = client.get(&url).send() {
                    if let Ok(locks_res) = res.json::<LocksResponse>() {
                        hub_locks = locks_res.locks;
                    }
                }
            }
        }

        // Mevcut HEAD'i oku (parent ve delta için)
        let head_path = format!("{}/HEAD", ctx.repo_path);
        let head_content = fs::read_to_string(&head_path).ok();

        let parent_id = if let Some(ref content) = head_content {
            if content.starts_with("ref: ") {
                let ref_path = content.trim_start_matches("ref: ").trim();
                fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path)).ok()
            } else {
                Some(content.trim().to_string())
            }
        } else {
            None
        };

        let mut parent_files = HashMap::new();
        if let Some(ref p_id) = parent_id {
            if let Ok(p_data) = ctx.get_object(p_id) {
                if let Ok(dec) = crate::storage::compression::decompress(&p_data) {
                    if let Ok(p_commit) = bincode::deserialize::<Commit>(&dec) {
                        if let Ok(files) = crate::core::sync::list_files_flattened(store, &p_commit.tree_hash, "") {
                            parent_files = files;
                        }
                    }
                }
            }
        }

        // Çalışma dizinini tara ve hiyerarşik yapıyı oluştur
        let mut root_tree = HierarchicalTreeBuilder::default();

        for entry in WalkDir::new(".")
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".something" && name != "target" && name != ".git" && name != ".hey"
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                let path_str = path.display().to_string().replace("./", "");

                // Uyarı: Başkası kilitlemiş mi?
                for lock in &hub_locks {
                    if lock.path == path_str {
                        println!("  [!] UYARI: '{}' dosyası {} tarafından kilitlenmiş!", path_str, lock.owner_name);
                    }
                }

                let mut content = Vec::new();
                std::fs::File::open(&path)?.read_to_end(&mut content)?;

                let file_size = content.len();
                let threshold = 10 * 1024 * 1024; // 10MB

                let (hash, entry_type, depth, _bin_data) = if file_size > threshold {
                    let chunk_results = crate::storage::chunker::Chunker::chunk_data(&content);
                    let mut chunk_hashes = Vec::new();
                    for (c_hash, c_data) in chunk_results {
                        let compressed = crate::storage::compression::compress(&c_data)?;
                        store.put(c_hash.as_bytes(), &compressed)?;
                        chunk_hashes.push(c_hash);
                    }
                    // Chunked dosyalar için delta şu an desteklenmiyor (basitlik için)
                    // TODO: Chunk-level delta logic
                    let bin = bincode::serialize(&crate::core::object_model::TreeEntry {
                         name: path_str.clone(),
                         hash: hash_data(&content),
                         entry_type: crate::core::object_model::EntryType::Blob,
                         mode: 0, 
                         delta_depth: 0,
                         is_chunked: true,
                         chunks: Some(chunk_hashes),
                    })?;
                    (hash_data(&content), crate::core::object_model::EntryType::Blob, 0, bin)
                } else {
                    let mut stored_hash = hash_data(&content);
                    let mut type_obj = crate::core::object_model::EntryType::Blob;
                    let mut final_bin = content.clone();
                    let mut final_depth = 0;

                    if let Some((base_hash, base_type, base_depth)) = parent_files.get(&path_str) {
                        const MAX_DELTA_DEPTH: u32 = 10;
                        
                        if *base_depth < MAX_DELTA_DEPTH {
                            if let Ok(base_data_raw) = ctx.get_object(base_hash) {
                                if let Ok(base_dec) = crate::storage::compression::decompress(&base_data_raw) {
                                    if let Ok(patch) = DeltaEngine::compute_delta(&base_dec, &content) {
                                        let delta_obj = DeltaObject {
                                            base_hash: base_hash.clone(),
                                            base_type: base_type.clone(),
                                            patch,
                                            final_size: file_size as u64,
                                        };
                                        let delta_bin = bincode::serialize(&delta_obj)?;
                                        
                                        // Eğer delta + sıkıştırma orijinalden anlamlı derecede küçükse delta kullan
                                        if delta_bin.len() < content.len() {
                                            stored_hash = hash_data(&delta_bin);
                                            type_obj = crate::core::object_model::EntryType::Delta;
                                            final_bin = delta_bin;
                                            final_depth = base_depth + 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    let compressed = crate::storage::compression::compress(&final_bin)?;
                    store.put(stored_hash.as_bytes(), &compressed)?;
                    (stored_hash, type_obj, final_depth, final_bin)
                };

                let mode = {
                    use std::os::unix::fs::PermissionsExt;
                    let metadata = entry.metadata()?;
                    metadata.permissions().mode()
                };

                root_tree.insert(&path_str, hash, entry_type, depth, mode);
            }
        }

        // Hiyerarşik Tree nesnelerini flush et
        let mut batch = Vec::new();
        let tree_hash = root_tree.flush(&mut batch)?;
        store.insert_batch(batch)?;

        // HEAD ve branch güncelleme mantığı (parent zaten yukarıda okundu)

        // Commit nesnesini oluştur
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let author = ctx
            .config
            .as_ref()
            .map(|c| c.user.name.clone())
            .unwrap_or_else(|| "Anonymous".to_string());

        let commit = Commit {
            parent_id,
            tree_hash: tree_hash.clone(),
            author,
            timestamp,
            message: message.clone(),
        };

        let bin = bincode::serialize(&commit)?;
        let commit_hash = hash_data(&bin);
        let compressed = crate::storage::compression::compress(&bin)?;
        store.put(commit_hash.as_bytes(), &compressed)?;

        // HEAD veya dalı güncelle
        if let Some(ref content) = head_content {
            if content.starts_with("ref: ") {
                let ref_path = content.trim_start_matches("ref: ").trim();
                fs::write(format!("{}/{}", ctx.repo_path, ref_path), &commit_hash)?;
            } else {
                let main_branch_path = format!("{}/refs/heads/main", ctx.repo_path);
                fs::create_dir_all(format!("{}/refs/heads", ctx.repo_path))?;
                fs::write(main_branch_path, &commit_hash)?;
                fs::write(&head_path, "ref: refs/heads/main")?;
            }
        } else {
            let main_branch_path = format!("{}/refs/heads/main", ctx.repo_path);
            fs::create_dir_all(format!("{}/refs/heads", ctx.repo_path))?;
            fs::write(main_branch_path, &commit_hash)?;
            fs::write(&head_path, "ref: refs/heads/main")?;
        }

        // Journal kaydı
        Journal::log(
            "save",
            json!({
                "commit_hash": commit_hash,
                "message": message,
                "tree_hash": tree_hash
            }),
        )?;

        println!("Kaydedildi: {}", commit_hash);
        Ok(())
    }
}

#[derive(Default)]
struct HierarchicalTreeBuilder {
    entries: std::collections::HashMap<String, BuilderEntry>,
}

enum BuilderEntry {
    Blob { hash: String, entry_type: crate::core::object_model::EntryType, depth: u32, mode: u32 },
    Tree(HierarchicalTreeBuilder),
}

impl HierarchicalTreeBuilder {
    fn insert(&mut self, path: &str, hash: String, entry_type: crate::core::object_model::EntryType, depth: u32, mode: u32) {
        let parts: Vec<&str> = path.split('/').collect();
        self.recursive_insert(&parts, hash, entry_type, depth, mode);
    }

    fn recursive_insert(&mut self, parts: &[&str], hash: String, entry_type: crate::core::object_model::EntryType, depth: u32, mode: u32) {
        if parts.is_empty() { return; }

        let part = parts[0].to_string();
        if parts.len() == 1 {
            self.entries.insert(part, BuilderEntry::Blob { hash, entry_type, depth, mode });
        } else {
            let node = self.entries.entry(part).or_insert_with(|| {
                BuilderEntry::Tree(HierarchicalTreeBuilder::default())
            });

            match node {
                BuilderEntry::Tree(next_tree) => {
                    next_tree.recursive_insert(&parts[1..], hash, entry_type, depth, mode);
                }
                _ => {
                    *node = BuilderEntry::Tree(HierarchicalTreeBuilder::default());
                    if let BuilderEntry::Tree(next_tree) = node {
                        next_tree.recursive_insert(&parts[1..], hash, entry_type, depth, mode);
                    }
                }
            }
        }
    }

    fn flush(&self, batch: &mut Vec<(Vec<u8>, Vec<u8>)>) -> Result<String> {
        use crate::core::object_model::{Tree, TreeEntry, EntryType};
        let mut entries = Vec::new();
        for (name, entry) in &self.entries {
            match entry {
                BuilderEntry::Blob { hash, entry_type, depth, mode } => {
                    entries.push(TreeEntry {
                        name: name.clone(),
                        hash: hash.clone(),
                        entry_type: entry_type.clone(),
                        mode: *mode,
                        delta_depth: *depth,
                        is_chunked: false, 
                        chunks: None,
                    });
                }
                BuilderEntry::Tree(t) => {
                    let h = t.flush(batch)?;
                    entries.push(TreeEntry {
                        name: name.clone(),
                        hash: h,
                        entry_type: EntryType::Tree,
                        mode: 0o755, // Klasörler için varsayılan izin
                        delta_depth: 0,
                        is_chunked: false,
                        chunks: None,
                    });
                }
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let tree = Tree { entries };
        let bin = bincode::serialize(&tree)?;
        let h = crate::crypto::hash::hash_data(&bin);
        let compressed = crate::storage::compression::compress(&bin)?;
        batch.push((h.as_bytes().to_vec(), compressed));
        Ok(h)
    }
}

#[derive(serde::Deserialize)]
struct LockInfo {
    path: String,
    owner_name: String,
}

#[derive(serde::Deserialize)]
struct LocksResponse {
    locks: Vec<LockInfo>,
}
