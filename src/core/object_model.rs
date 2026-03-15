use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Commit {
    pub parent_id: Option<String>,
    pub tree_hash: String,
    pub author: String,
    pub timestamp: u64,
    pub message: String,
}

impl Commit {
    #[allow(dead_code)]
    pub fn verify_integrity(&self, current_hash: &str) -> bool {
        if let Ok(bin) = bincode::serialize(self) {
            let calculated = crate::crypto::hash::hash_data(&bin);
            return calculated == current_hash;
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tree {
    /// Bu ağacın (klasörün) altındaki dosyalar ve alt klasörler
    pub entries: Vec<TreeEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EntryType {
    Blob, // Dosya (veya chunked dosya başlığı)
    Tree, // Alt Klasör
    Delta, // Yeni: Başka bir objenin üzerine uygulanacak fark
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub hash: String,
    pub entry_type: EntryType,
    pub mode: u32,
    pub delta_depth: u32, // Yeni: Delta zinciri derinliği
    pub is_chunked: bool,
    pub chunks: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub target_id: String,
    pub name: String,
    pub tagger: String,
    pub timestamp: u64,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeltaObject {
    pub base_hash: String,
    pub base_type: EntryType, // Yeni: Base objenin tipi (Blob veya Delta)
    pub patch: Vec<u8>,
    pub final_size: u64,
}
