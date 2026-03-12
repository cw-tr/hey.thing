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
        // En basit haliyle, nesneyi tekrar serialize edip hash'ini kontrol ederiz
        // Phase 4'te bu Merkle zincirine ve dijital imzaya bağlanacak.
        if let Ok(json) = serde_json::to_vec(self) {
            let calculated = crate::crypto::hash::hash_data(&json);
            return calculated == current_hash;
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub hash: String,
    pub is_dir: bool,
    pub is_chunked: bool,
    pub chunks: Option<Vec<String>>,
}
