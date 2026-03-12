use crate::storage::kv_store::KvStore;
use crate::core::object_model::{Commit, Tree};
use anyhow::Result;
use std::collections::HashSet;

/// Senkronizasyon sürecinde transfer edilecek veriyi tutan paket
pub struct DeltaPackage {
    pub commits: Vec<(String, Vec<u8>)>,
    pub trees: Vec<(String, Vec<u8>)>,
    pub blobs: Vec<(String, Vec<u8>)>,
}

/// İki branch arasındaki ortak ancestor'un (kesişim noktası) hash'ini bulur
pub fn find_common_ancestor(
    store: &KvStore,
    head_a: &str, // Lokalde bulunan
    head_b: &str  // Uzaktan gelen
) -> Result<Option<String>> {
    let mut history_a = HashSet::new();
    let mut current_a = head_a.to_string();

    // 1. A dalının tüm geçmişini topla
    loop {
        history_a.insert(current_a.clone());
        if let Some(json) = store.get(current_a.as_bytes())? {
            let commit: Commit = serde_json::from_slice(&json)?;
            if let Some(parent) = commit.parent_id {
                current_a = parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // 2. B dalında yukarı doğru çıkıp A'nın geçmişiyle ilk kesişimi bul
    let mut current_b = head_b.to_string();
    loop {
        if history_a.contains(&current_b) {
            return Ok(Some(current_b));
        }

        if let Some(json) = store.get(current_b.as_bytes())? {
            let commit: Commit = serde_json::from_slice(&json)?;
            if let Some(parent) = commit.parent_id {
                current_b = parent;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(None)
}

/// Ortak noktadan itibaren hedef head'e kadar olan farklı (yeni) commit'leri, tree'leri ve blob'ları toplar.
/// Bu fonksiyon "benim lokalimi karşıya aktarırken kullanılacak paketi" hazırlar.
pub fn compute_delta(
    store: &KvStore,
    target_head: &str,
    common_ancestor: Option<&str>
) -> Result<DeltaPackage> {
    let mut commits = Vec::new();
    let mut trees = Vec::new();
    let mut blobs = Vec::new();

    let mut current_hash = target_head.to_string();
    let mut visited_trees = HashSet::new();
    let mut visited_blobs = HashSet::new();

    // Ancestor'a kadar (veya köke kadar) commit zincirini tara
    loop {
        if let Some(ancest) = common_ancestor {
            if current_hash == ancest {
                break;
            }
        }

        if let Some(commit_json) = store.get(current_hash.as_bytes())? {
            commits.push((current_hash.clone(), commit_json.clone()));
            
            let commit: Commit = serde_json::from_slice(&commit_json)?;
            collect_tree_recursive(store, &commit.tree_hash, &mut trees, &mut blobs, &mut visited_trees, &mut visited_blobs)?;

            if let Some(parent) = commit.parent_id {
                current_hash = parent;
            } else {
                break; // İlk commit'te dur
            }
        } else {
            break;
        }
    }

    Ok(DeltaPackage { commits, trees, blobs })
}

/// Recursive olarak bir tree içindeki alt tree'leri ve blob'ları toplar
fn collect_tree_recursive(
    store: &KvStore,
    tree_hash: &str,
    trees: &mut Vec<(String, Vec<u8>)>,
    blobs: &mut Vec<(String, Vec<u8>)>,
    visited_trees: &mut HashSet<String>,
    visited_blobs: &mut HashSet<String>,
) -> Result<()> {
    if visited_trees.contains(tree_hash) {
        return Ok(());
    }

    if let Some(tree_json) = store.get(tree_hash.as_bytes())? {
        visited_trees.insert(tree_hash.to_string());
        trees.push((tree_hash.to_string(), tree_json.clone()));

        let tree_obj: Tree = serde_json::from_slice(&tree_json)?;
        for entry in tree_obj.entries {
            if entry.is_dir {
                collect_tree_recursive(store, &entry.hash, trees, blobs, visited_trees, visited_blobs)?;
            } else {
                if !visited_blobs.contains(&entry.hash) {
                    if let Some(blob_data) = store.get(entry.hash.as_bytes())? {
                        visited_blobs.insert(entry.hash.clone());
                        blobs.push((entry.hash.clone(), blob_data));
                    }
                }
                
                // Chunklanmış dosya ise, chunkları da blob statüsünde al
                if let Some(chunk_hashes) = entry.chunks {
                    for chunk_hash in chunk_hashes {
                        if !visited_blobs.contains(&chunk_hash) {
                            if let Some(chunk_data) = store.get(chunk_hash.as_bytes())? {
                                visited_blobs.insert(chunk_hash.clone());
                                blobs.push((chunk_hash.clone(), chunk_data));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
