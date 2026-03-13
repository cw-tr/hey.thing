use crate::storage::kv_store::KvStore;
use crate::core::object_model::{Commit, Tree};
use anyhow::Result;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Senkronizasyon sürecinde transfer edilecek veriyi tutan paket
#[derive(Serialize, Deserialize, Debug)]
pub struct DeltaPackage {
    pub commits: Vec<(String, Vec<u8>)>,
    pub trees: Vec<(String, Vec<u8>)>,
    pub blobs: Vec<(String, Vec<u8>)>,
}

/// İki branch arasındaki ortak ancestor'un (kesişim noktası) hash'ini bulur
pub fn find_common_ancestor(
    store: &KvStore,
    head_a: &str,
    head_b: &str
) -> Result<Option<String>> {
    find_common_ancestor_cross(store, head_a, store, head_b)
}

pub fn find_common_ancestor_cross(
    store_a: &KvStore,
    head_a: &str,
    store_b: &KvStore,
    head_b: &str
) -> Result<Option<String>> {
    let mut history_a = HashSet::new();
    let mut current_a = head_a.to_string();

    // 1. A dalının tüm geçmişini topla (store_a kullanarak)
    loop {
        if current_a.is_empty() { break; }
        history_a.insert(current_a.clone());
        if let Some(json) = store_a.get(current_a.as_bytes())? {
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

    // 2. B dalında yukarı doğru çıkıp (store_b kullanarak) A'nın geçmişiyle ilk kesişimi bul
    let mut current_b = head_b.to_string();
    loop {
        if current_b.is_empty() { break; }
        if history_a.contains(&current_b) {
            return Ok(Some(current_b));
        }

        if let Some(json) = store_b.get(current_b.as_bytes())? {
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

/// Çalışma dizinini belirli bir tree state'ine çeker (Checkout)
pub fn apply_checkout(
    store: &KvStore,
    tree_hash: &str,
    repo_root: &std::path::Path,
) -> Result<()> {
    let files = list_files_flattened(store, tree_hash, "")?;
    
    // 1. Yeni tree'deki dosyaları yaz/güncelle
    for (path, blob_hash) in &files {
        let full_path = repo_root.join(path);
        
        let blob_data = store.get(blob_hash.as_bytes())?
            .ok_or_else(|| anyhow::anyhow!("Blob bulunamadı: {}", blob_hash))?;
        
        // Sıkıştırmayı aç
        let content = crate::storage::compression::decompress(&blob_data)?;
        
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Eğer dosya zaten varsa ve içeriği aynıysa yazmaya gerek yok (Opsiyonel optimizasyon)
        std::fs::write(full_path, content)?;
    }

    // TODO: Mevcut olup ama yeni tree'de olmayan dosyaları sil (Tracked file cleanup)
    
    Ok(())
}

/// Gerçek satır bazlı 3-way merge algoritması.
/// similar crate'in Myers diff motorunu kullanarak Base→Local ve Base→Remote
/// değişiklik kümelerini satır satır birleştirir.
/// Sadece her iki tarafın da aynı satır bölgesini değiştirdiği durumlarda
/// conflict marker eklenir; tek taraflı değişiklikler otomatik birleşir.
pub fn merge_content_3way(base: &str, local: &str, remote: &str) -> (String, bool) {
    use similar::{Algorithm, ChangeTag, TextDiff};

    // Hızlı yollar
    if local == remote { return (local.to_string(), false); }
    if local == base   { return (remote.to_string(), false); }
    if remote == base  { return (local.to_string(), false); }

    let base_lines: Vec<&str>   = base.lines().collect();
    let local_lines: Vec<&str>  = local.lines().collect();
    let remote_lines: Vec<&str> = remote.lines().collect();

    // Base'den Local'e -> hangi base satırları değişti?
    // (true = bu base satırı local'de silinmiş veya değiştirilmiş)
    let mut local_changed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    {
        let diff = TextDiff::configure()
            .algorithm(Algorithm::Myers)
            .diff_lines(base, local);
        let mut base_idx = 0usize;
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal  => { base_idx += 1; }
                ChangeTag::Delete => { local_changed.insert(base_idx); base_idx += 1; }
                ChangeTag::Insert => {}
            }
        }
    }

    // Base'den Remote'a -> hangi base satırları değişti?
    let mut remote_changed: std::collections::HashSet<usize> = std::collections::HashSet::new();
    {
        let diff = TextDiff::configure()
            .algorithm(Algorithm::Myers)
            .diff_lines(base, remote);
        let mut base_idx = 0usize;
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal  => { base_idx += 1; }
                ChangeTag::Delete => { remote_changed.insert(base_idx); base_idx += 1; }
                ChangeTag::Insert => {}
            }
        }
    }

    // Her iki taraf da aynı satırı değiştirdiyse → conflict; aksi halde tek taraf kazanır
    let mut result       = String::new();
    let mut has_conflict = false;
    let mut base_idx     = 0usize;
    let mut local_idx    = 0usize;
    let mut remote_idx   = 0usize;

    while base_idx < base_lines.len() || local_idx < local_lines.len() || remote_idx < remote_lines.len() {
        let l_changed = local_changed.contains(&base_idx);
        let r_changed = remote_changed.contains(&base_idx);

        if base_idx >= base_lines.len() {
            // Base bitti, kalan eklentiler
            if local_idx < local_lines.len() {
                result.push_str(local_lines[local_idx]);
                result.push('\n');
                local_idx += 1;
            } else if remote_idx < remote_lines.len() {
                result.push_str(remote_lines[remote_idx]);
                result.push('\n');
                remote_idx += 1;
            }
            continue;
        }

        if !l_changed && !r_changed {
            // İki taraf da değiştirmemiş → base'i yaz
            result.push_str(base_lines[base_idx]);
            result.push('\n');
            base_idx  += 1;
            if local_idx  < local_lines.len()  && local_lines[local_idx]  == base_lines[base_idx.saturating_sub(1)] { local_idx  += 1; }
            if remote_idx < remote_lines.len() && remote_lines[remote_idx] == base_lines[base_idx.saturating_sub(1)] { remote_idx += 1; }
        } else if l_changed && !r_changed {
            // Sadece local değiştirmiş → local kazanır
            if local_idx < local_lines.len() {
                result.push_str(local_lines[local_idx]);
                result.push('\n');
                local_idx += 1;
            }
            remote_idx = remote_idx.saturating_add(1).min(remote_lines.len());
            base_idx  += 1;
        } else if !l_changed && r_changed {
            // Sadece remote değiştirmiş → remote kazanır
            if remote_idx < remote_lines.len() {
                result.push_str(remote_lines[remote_idx]);
                result.push('\n');
                remote_idx += 1;
            }
            local_idx = local_idx.saturating_add(1).min(local_lines.len());
            base_idx += 1;
        } else {
            // Her iki taraf da değiştirmiş → CONFLICT
            has_conflict = true;
            result.push_str("<<<<<<< YOURS (LOCAL)\n");
            if local_idx < local_lines.len() {
                result.push_str(local_lines[local_idx]);
                result.push('\n');
                local_idx += 1;
            }
            result.push_str("=======\n");
            if remote_idx < remote_lines.len() {
                result.push_str(remote_lines[remote_idx]);
                result.push('\n');
                remote_idx += 1;
            }
            result.push_str(">>>>>>> THEIRS (REMOTE)\n");
            base_idx += 1;
        }
    }

    (result, has_conflict)
}

/// İki commit arasında merge işlemini gerçekleştirir ve çalışma dizinine yansıtır
pub fn perform_merge(
    store: &KvStore,
    repo_root: &std::path::Path,
    local_head: &str,
    remote_head: &str,
    ancestor: &str,
) -> Result<()> {
    let base_commit: Commit = serde_json::from_slice(&store.get(ancestor.as_bytes())?.ok_or_else(|| anyhow::anyhow!("Base commit bulunamadı"))?)?;
    let local_commit: Commit = serde_json::from_slice(&store.get(local_head.as_bytes())?.ok_or_else(|| anyhow::anyhow!("Local commit bulunamadı"))?)?;
    let remote_commit: Commit = serde_json::from_slice(&store.get(remote_head.as_bytes())?.ok_or_else(|| anyhow::anyhow!("Remote commit bulunamadı"))?)?;

    let candidates = find_merge_candidates(
        store,
        &base_commit.tree_hash,
        &local_commit.tree_hash,
        &remote_commit.tree_hash,
    )?;

    println!("{} dosya üzerinde merge analizi yapılıyor...", candidates.len());
    
    // Wasm AST Eklentilerini Yükle
    let mut lang_registry = crate::plugins::lang_registry::LangRegistry::new();
    let lang_paths = crate::plugins::get_plugin_search_paths("langs");
    lang_registry.load_plugins_from_dirs(&lang_paths);

    for (path, (base_hash, local_hash, remote_hash)) in candidates {
        let base_data = if let Some(h) = base_hash { 
            let data = store.get(h.as_bytes())?.unwrap_or_default();
            String::from_utf8_lossy(&crate::storage::compression::decompress(&data)?).to_string()
        } else { String::new() };

        let local_data = if let Some(h) = local_hash { 
            let data = store.get(h.as_bytes())?.unwrap_or_default();
            String::from_utf8_lossy(&crate::storage::compression::decompress(&data)?).to_string()
        } else { String::new() };

        let remote_data = if let Some(h) = remote_hash { 
            let data = store.get(h.as_bytes())?.unwrap_or_default();
            String::from_utf8_lossy(&crate::storage::compression::decompress(&data)?).to_string()
        } else { String::new() };

        let mut has_conflict = false;
        let mut final_content;

        if let Some(merger) = lang_registry.get_merger(&path) {
            match merger.merge(&base_data, &local_data, &remote_data) {
                Ok(result) => {
                    final_content = result;
                }
                Err(e) => {
                    println!("  [-] Olası AST Merging Hatası ({}): {}", merger.name(), e);
                    let (res, conf) = merge_content_3way(&base_data, &local_data, &remote_data);
                    final_content = res;
                    has_conflict = conf;
                }
            }
        } else {
            let (res, conf) = merge_content_3way(&base_data, &local_data, &remote_data);
            final_content = res;
            has_conflict = conf;
        }
        
        if has_conflict {
            println!("  [!] ÇAKIŞMA (Conflict): {}. Görsel asistan başlatılıyor...", path);
            match crate::tui::conflict_resolver::resolve_conflict_interactive(&path, &base_data, &local_data, &remote_data) {
                Ok(res) => {
                    if res.resolved {
                        final_content = res.content;
                        println!("  [+] Görsel asistan ile çözüldü: {}", path);
                    } else {
                        println!("  [!] Çatışma çözülmedi, çakışma işaretçileriyle kaydediliyor: {}", path);
                    }
                }
                Err(e) => {
                    eprintln!("  [!] TUI Başlatılamadı: {}. Klasik modla devam ediliyor.", e);
                }
            }
        } else {
            println!("  [+] Birleştirildi: {}", path);
        }
        
        let full_path = repo_root.join(&path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full_path, final_content)?;
    }

    Ok(())
}

/// İki Tree objesini karşılaştırıp birleştirilmeleri gereken dosyaları belirler
pub fn find_merge_candidates(
    store: &KvStore,
    base_tree_hash: &str,
    local_tree_hash: &str,
    remote_tree_hash: &str,
) -> Result<std::collections::HashMap<String, (Option<String>, Option<String>, Option<String>)>> {
    let mut results = std::collections::HashMap::new();
    
    let base_files = list_files_flattened(store, base_tree_hash, "")?;
    let local_files = list_files_flattened(store, local_tree_hash, "")?;
    let remote_files = list_files_flattened(store, remote_tree_hash, "")?;

    let mut all_paths = HashSet::new();
    for p in base_files.keys() { all_paths.insert(p); }
    for p in local_files.keys() { all_paths.insert(p); }
    for p in remote_files.keys() { all_paths.insert(p); }

    for path in all_paths {
        let b = base_files.get(path).cloned();
        let l = local_files.get(path).cloned();
        let r = remote_files.get(path).cloned();
        
        // Eğer üç taraftan en az ikisi farklıysa merge adayıdır
        if l != r {
            results.insert(path.clone(), (b, l, r));
        }
    }

    Ok(results)
}

fn list_files_flattened(store: &KvStore, tree_hash: &str, prefix: &str) -> Result<std::collections::HashMap<String, String>> {
    let mut files = std::collections::HashMap::new();
    if let Some(json) = store.get(tree_hash.as_bytes())? {
        let tree: Tree = serde_json::from_slice(&json)?;
        for entry in tree.entries {
            let full_path = if prefix.is_empty() { entry.name.clone() } else { format!("{}/{}", prefix, entry.name) };
            if entry.is_dir {
                let sub_files = list_files_flattened(store, &entry.hash, &full_path)?;
                files.extend(sub_files);
            } else {
                files.insert(full_path, entry.hash);
            }
        }
    }
    Ok(files)
}
