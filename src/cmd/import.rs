use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::crypto::hash::hash_data;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct ImportVerb;

impl ImportVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for ImportVerb {
    fn name(&self) -> &str {
        "import"
    }

    fn help(&self) -> &str {
        "Dışarıdan (örn: Git) proje aktarımı yapar"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.len() >= 2 && args[0] == "--from-git" {
            let git_source = &args[1];
            
            if is_url(git_source) {
                let temp_dir = tempfile::Builder::new().prefix("hey_import_").tempdir()?;
                let temp_path = temp_dir.path().to_str().ok_or_else(|| anyhow!("Invalid temp path"))?;
                
                println!("🌍 Uzak repo algılandı: {}", git_source);
                println!("  [+] Geçici olarak klonlanıyor (bu işlem ağ hızınıza bağlıdır)...");
                
                let status = Command::new("git")
                    .args(["clone", "--bare", "--quiet", git_source, temp_path])
                    .status()?;
                
                if !status.success() {
                    return Err(anyhow!("Git klonlama başarısız oldu."));
                }
                
                return import_from_git(ctx, temp_path);
            } else {
                return import_from_git(ctx, git_source);
            }
        }

        println!("Kullanım: hey import --from-git <git-repo-yolu-veya-URL>");
        Ok(())
    }
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("git@")
}

use std::io::{Write, BufReader, BufRead, Read};
use std::process::{Stdio};
use std::collections::HashMap;
use rayon::prelude::*;

fn import_from_git(ctx: &ThingContext, git_repo_path: &str) -> Result<()> {
    let repo_path = Path::new(git_repo_path);
    let git_dir = if repo_path.join(".git").exists() {
        repo_path.join(".git")
    } else {
        repo_path.to_path_buf() // Bare repo durumu
    };

    if !git_dir.join("objects").exists() {
        return Err(anyhow!("'{}' geçerli bir Git reposu değil (objects dizini bulunamadı).", git_repo_path));
    }

    let store = ctx.store.as_ref()
        .ok_or_else(|| anyhow!("Repo başlatılmamış. Önce 'hey init' çalıştırın."))?;

    println!("🚀 [Stateful Engine] Git migration başlatılıyor: {}", git_repo_path);

    // Sistem kilitlenmesini önlemek için en az 1 çekirdeği boşta bırak
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1);
    
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| anyhow!("Thread pool oluşturulamadı: {}", e))?;

    println!("  [+] {} çekirdek ile paralel işlem kapasitesi ayrıldı.", threads);

    // 1. Git süreçlerini hazırla
    let mut cat_batch = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(git_repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut cat_stdin = cat_batch.stdin.take().unwrap();
    let mut cat_stdout = BufReader::new(cat_batch.stdout.take().unwrap());

    // 2. High-Performance Stream
    let mut log_child = Command::new("git")
        .args(["log", "--reverse", "--all", "--raw", "-r", "--format=COMMIT|%H|%an|%at|%P|%s"])
        .current_dir(git_repo_path)
        .stdout(Stdio::piped())
        .spawn()?;

    let log_stdout = BufReader::new(log_child.stdout.take().unwrap());
    
    let mut blob_map: HashMap<String, String> = HashMap::new();
    let mut commit_map: HashMap<String, String> = HashMap::new();
    
    // Artık kök dizini tutuyoruz, hiyerarşi bunun içine recursive kurulacak
    let mut root_tree = HierarchicalTree::default();
    
    let mut pending: Option<PendingCommit> = None;
    let mut commit_changes: Vec<FileChange> = Vec::new();
    let mut imported_count = 0;

    println!("  [+] İşlem başlatıldı...");

    let mut log_reader = log_stdout;
    let mut buf = Vec::new();

    loop {
        buf.clear();
        let bytes_read = log_reader.read_until(b'\n', &mut buf)?;
        if bytes_read == 0 {
            // EOF - Son commit'i işle
            if let Some(p) = pending.take() {
                pool.install(|| {
                    process_commit_changes(
                        store,
                        &mut root_tree,
                        &mut blob_map,
                        &mut commit_changes,
                        &mut cat_stdin,
                        &mut cat_stdout
                    )?;
                    let git_hash = p.hash.clone();
                    let mut batch = Vec::new();
                    let h = flush_hierarchical_tree(store, &mut root_tree, p, &commit_map, &mut batch)?;
                    store.insert_batch(batch)?;
                    commit_map.insert(git_hash, h);
                    imported_count += 1;
                    Ok::<(), anyhow::Error>(())
                })?;
            }
            break;
        }

        let line = String::from_utf8_lossy(&buf).trim().to_string();
        if line.is_empty() { continue; }

        if line.starts_with("COMMIT|") {
            // Önceki commit'i kaydet
            if let Some(p) = pending.take() {
                pool.install(|| {
                    process_commit_changes(
                        store,
                        &mut root_tree,
                        &mut blob_map,
                        &mut commit_changes,
                        &mut cat_stdin,
                        &mut cat_stdout
                    )?;
                    let git_hash = p.hash.clone();
                    let mut batch = Vec::new();
                    let h = flush_hierarchical_tree(store, &mut root_tree, p, &commit_map, &mut batch)?;
                    store.insert_batch(batch)?;
                    commit_map.insert(git_hash, h);
                    imported_count += 1;
                    if imported_count % 1000 == 0 {
                        println!("  [%] {} commit tamamlandı...", imported_count);
                    }
                    Ok::<(), anyhow::Error>(())
                })?;
            }

            let parts: Vec<&str> = line["COMMIT|".len()..].splitn(5, '|').collect();
            if parts.len() < 5 { continue; }

            pending = Some(PendingCommit {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                timestamp: parts[2].parse().unwrap_or(0),
                parent_git: parts[3].split_whitespace().next().map(|s| s.to_string()),
                message: parts[4].to_string(),
            });
        } else if line.starts_with(':') {
            let tab_split: Vec<&str> = line.splitn(2, '\t').collect();
            if tab_split.len() < 2 { continue; }
            let path = tab_split[1];
            let meta: Vec<&str> = tab_split[0].split_whitespace().collect();
            if meta.len() < 5 { continue; }
            
            let mode_str = meta[0].trim_start_matches(':');
            let mode = u32::from_str_radix(mode_str, 8).unwrap_or(0o100644);
            let dst_hash = meta[3];
            let status = meta[4];

            commit_changes.push(FileChange {
                path: path.to_string(),
                git_hash: dst_hash.to_string(),
                is_delete: status.starts_with('D'),
                mode,
            });
        }
    }

    // 3. Referansları Aktar
    println!("  [+] Referanslar aktarılıyor...");
    let refs_output = Command::new("git")
        .args(["for-each-ref", "--format=%(refname)|%(objectname)|%(objecttype)|%(taggername)|%(taggerdate:unix)|%(contents:subject)"])
        .current_dir(git_repo_path)
        .output()?;
    
    let refs_text = String::from_utf8_lossy(&refs_output.stdout);
    for line in refs_text.lines() {
        let p: Vec<&str> = line.split('|').collect();
        if p.len() < 2 { continue; }
        let ref_name = p[0];
        let git_h = p[1];
        
        if ref_name.starts_with("refs/heads/") {
            if let Some(h) = commit_map.get(git_h) {
                let branch = &ref_name["refs/heads/".len()..];
                let path = format!("{}/refs/heads", ctx.repo_path);
                fs::create_dir_all(&path)?;
                fs::write(format!("{}/{}", path, branch), h)?;
            }
        } else if ref_name.starts_with("refs/tags/") {
             if let Some(h) = commit_map.get(git_h) {
                let tag = &ref_name["refs/tags/".len()..];
                let path = format!("{}/refs/tags", ctx.repo_path);
                fs::create_dir_all(&path)?;
                fs::write(format!("{}/{}", path, tag), h)?;
            }
        }
    }

    // 4. HEAD
    let head_ref = Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).current_dir(git_repo_path).output()?;
    let current_branch = String::from_utf8_lossy(&head_ref.stdout).trim().to_string();
    if current_branch != "HEAD" {
        fs::write(format!("{}/HEAD", ctx.repo_path), format!("ref: refs/heads/{}", current_branch))?;
    }

    println!("\n✨ Migration tamamlandı. Toplam {} commit aktarıldı.", imported_count);
    Ok(())
}

/// Hiyerarşik ağaç yapısını bellekte tutmak için yardımcı yapı
#[derive(Default)]
struct HierarchicalTree {
    entries: HashMap<String, HierarchicalEntry>,
    cached_hash: Option<String>,
}

enum HierarchicalEntry {
    Blob(String, u32), // Hash, Mode
    Tree(HierarchicalTree),
    Chunked(String, Vec<String>, u32), // Head Hash, Chunk Hashes, Mode
}

struct FileChange {
    path: String,
    git_hash: String,
    is_delete: bool,
    mode: u32,
}

fn process_commit_changes(
    store: &crate::storage::kv_store::KvStore,
    root: &mut HierarchicalTree,
    blob_map: &mut HashMap<String, String>,
    changes: &mut Vec<FileChange>,
    cat_stdin: &mut std::process::ChildStdin,
    cat_stdout: &mut BufReader<std::process::ChildStdout>,
) -> Result<()> {
    let mut to_fetch = Vec::new();
    for change in changes.drain(..) {
        if change.is_delete {
            remove_from_tree(root, &change.path);
        } else {
            if let Some(h) = blob_map.get(&change.git_hash) {
                insert_into_tree(root, &change.path, HierarchicalEntry::Blob(h.clone(), change.mode));
            } else if change.git_hash != "0000000000000000000000000000000000000000" {
                to_fetch.push(change);
            }
        }
    }

    if to_fetch.is_empty() { return Ok(()); }

    // 2. Eksik blob'ları Git'ten çek (Batched I/O)
    let mut fetched_blobs = Vec::new();
    for change in &to_fetch {
        cat_stdin.write_all(format!("{}\n", change.git_hash).as_bytes())?;
    }
    cat_stdin.flush()?;

    for _ in 0..to_fetch.len() {
        let mut head = String::new();
        let bytes_read = cat_stdout.read_line(&mut head)?;
        if bytes_read == 0 { break; }

        let h_parts: Vec<&str> = head.split_whitespace().collect();
        if h_parts.len() >= 3 {
            let size: usize = h_parts[2].parse().unwrap_or(0);
            let mut content = vec![0u8; size];
            cat_stdout.read_exact(&mut content)?;
            let mut dummy = [0u8; 1];
            cat_stdout.read_exact(&mut dummy)?;
            fetched_blobs.push(content);
        }
    }

    // 3. Paralel Havuzda Hash + Sıkıştırma + Yazma
    let processed: Vec<String> = fetched_blobs.into_par_iter().map(|content| {
        let h = hash_data(&content);
        let compressed = crate::storage::compression::compress(&content).unwrap_or_default();
        let _ = store.put(h.as_bytes(), &compressed);
        h
    }).collect();

    // 4. Sonuçları haritaya ve ağaca ekle
    for (i, hey_h) in processed.into_iter().enumerate() {
        if i < to_fetch.len() {
            let change = &to_fetch[i];
            blob_map.insert(change.git_hash.clone(), hey_h.clone());
            insert_into_tree(root, &change.path, HierarchicalEntry::Blob(hey_h, change.mode));
        }
    }

    Ok(())
}

fn insert_into_tree(tree: &mut HierarchicalTree, path: &str, entry: HierarchicalEntry) {
    let parts: Vec<&str> = path.split('/').collect();
    recursive_insert(tree, &parts, entry);
}

fn recursive_insert(tree: &mut HierarchicalTree, parts: &[&str], entry: HierarchicalEntry) {
    if parts.is_empty() { return; }
    
    // Değişiklik olduğu için bu seviyenin hash'i artık geçersiz
    tree.cached_hash = None;
    
    let part = parts[0].to_string();
    if parts.len() == 1 {
        tree.entries.insert(part, entry);
    } else {
        let node = tree.entries.entry(part).or_insert_with(|| {
            HierarchicalEntry::Tree(HierarchicalTree::default())
        });
        
        match node {
            HierarchicalEntry::Tree(next_tree) => {
                recursive_insert(next_tree, &parts[1..], entry);
            }
            _ => {
                // Çakışma: dosya iken klasör olma durumu, üzerine yaz
                *node = HierarchicalEntry::Tree(HierarchicalTree::default());
                if let HierarchicalEntry::Tree(next_tree) = node {
                    recursive_insert(next_tree, &parts[1..], entry);
                }
            }
        }
    }
}

fn remove_from_tree(tree: &mut HierarchicalTree, path: &str) {
    let parts: Vec<&str> = path.split('/').collect();
    recursive_remove(tree, &parts);
}

fn recursive_remove(tree: &mut HierarchicalTree, parts: &[&str]) {
    if parts.is_empty() { return; }
    
    // Değişiklik olduğu için bu seviyenin hash'i artık geçersiz
    tree.cached_hash = None;
    
    let part = parts[0];
    if parts.len() == 1 {
        tree.entries.remove(part);
    } else {
        if let Some(HierarchicalEntry::Tree(next_tree)) = tree.entries.get_mut(part) {
            recursive_remove(next_tree, &parts[1..]);
            // Eğer alt klasör boşaldıysa, onu silebiliriz (opsiyonel)
            if next_tree.entries.is_empty() {
                tree.entries.remove(part);
            }
        }
    }
}

fn flush_hierarchical_tree(
    _store: &crate::storage::kv_store::KvStore,
    root: &mut HierarchicalTree,
    p: PendingCommit,
    commit_map: &HashMap<String, String>,
    batch: &mut Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<String> {
    let tree_hash = recursive_flush_tree(root, batch)?;
    
    let parent_id = p.parent_git.and_then(|g| commit_map.get(&g).cloned());
    let commit = crate::core::object_model::Commit {
        parent_id,
        tree_hash,
        author: p.author,
        timestamp: p.timestamp,
        message: p.message,
    };
    
    let bin = bincode::serialize(&commit)?;
    let compressed = crate::storage::compression::compress(&bin)?;
    let commit_hash = hash_data(&bin);
    batch.push((commit_hash.as_bytes().to_vec(), compressed));
    
    Ok(commit_hash)
}

fn recursive_flush_tree(tree: &mut HierarchicalTree, batch: &mut Vec<(Vec<u8>, Vec<u8>)>) -> Result<String> {
    if let Some(ref h) = tree.cached_hash {
        return Ok(h.clone());
    }

    use crate::core::object_model::{Tree, TreeEntry, EntryType};

    // 1. Alt ağaçları Rayon ile paralel olarak işlet
    let sub_results: Vec<Result<(String, String, Vec<(Vec<u8>, Vec<u8>)>)>> = tree.entries.par_iter_mut().filter_map(|(name, entry)| {
        match entry {
            HierarchicalEntry::Tree(t) => {
                let name_clone = name.clone();
                Some(rayon::scope(move |_| {
                    let mut sub_batch = Vec::new();
                    recursive_flush_tree(t, &mut sub_batch).map(|h| (name_clone, h, sub_batch))
                }))
            }
            _ => None,
        }
    }).collect();

    // Hataları kontrol et ve alt batch'leri topla
    let mut sub_tree_hashes = HashMap::new();
    for res in sub_results {
        let (name, h, sub_batch) = res?;
        sub_tree_hashes.insert(name, h);
        batch.extend(sub_batch);
    }

    // 2. Entries listesini oluştur
    let mut entries = Vec::new();
    for (name, entry) in &tree.entries {
        match entry {
            HierarchicalEntry::Blob(h, mode) => {
                entries.push(TreeEntry {
                    name: name.clone(),
                    hash: h.clone(),
                    entry_type: EntryType::Blob,
                    mode: *mode,
                    delta_depth: 0,
                    is_chunked: false,
                    chunks: None,
                });
            }
            HierarchicalEntry::Tree(_) => {
                let h = sub_tree_hashes.get(name).unwrap().clone();
                entries.push(TreeEntry {
                    name: name.clone(),
                    hash: h,
                    entry_type: EntryType::Tree,
                    mode: 0o755,
                    delta_depth: 0,
                    is_chunked: false,
                    chunks: None,
                });
            }
            HierarchicalEntry::Chunked(h, chunks, mode) => {
               entries.push(TreeEntry {
                    name: name.clone(),
                    hash: h.clone(),
                    entry_type: EntryType::Blob,
                    mode: *mode,
                    delta_depth: 0,
                    is_chunked: true,
                    chunks: Some(chunks.clone()),
                });
            }
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));

    let tree_obj = Tree { entries };
    let bin = bincode::serialize(&tree_obj)?;
    let hash = hash_data(&bin);
    
    let compressed = crate::storage::compression::compress(&bin)?;
    batch.push((hash.as_bytes().to_vec(), compressed));
    
    tree.cached_hash = Some(hash.clone());
    Ok(hash)
}

struct PendingCommit {
    hash: String,
    author: String,
    timestamp: u64,
    parent_git: Option<String>,
    message: String,
}
