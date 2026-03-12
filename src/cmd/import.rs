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

    fn aliases(&self) -> &[&str] {
        &[]
    }

    fn help(&self) -> &str {
        "Dışarıdan (örn: Git) proje aktarımı yapar"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.len() >= 2 && args[0] == "--from-git" {
            let git_path = &args[1];
            return import_from_git(ctx, git_path);
        }

        println!("Kullanım: hey import --from-git <git-repo-yolu>");
        Ok(())
    }
}

fn import_from_git(ctx: &ThingContext, git_repo_path: &str) -> Result<()> {
    let git_dir = Path::new(git_repo_path).join(".git");
    if !git_dir.exists() {
        return Err(anyhow!("'{}' bir Git reposu değil (.git dizini bulunamadı).", git_repo_path));
    }

    let store = ctx.store.as_ref()
        .ok_or_else(|| anyhow!("Repo başlatılmamış. Önce 'hey init' çalıştırın."))?;

    println!("Git migration başlatılıyor: {}", git_repo_path);

    // 1. Git log ile commit listesini al (en eskiden en yeniye)
    let output = Command::new("git")
        .args(["log", "--reverse", "--format=%H|%an|%at|%s", "--all"])
        .current_dir(git_repo_path)
        .output()?;

    if !output.status.success() {
        return Err(anyhow!("Git log çalıştırılamadı: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let log_output = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<&str> = log_output.lines().collect();

    if commits.is_empty() {
        println!("Git reposunda commit bulunamadı.");
        return Ok(());
    }

    println!("{} commit bulundu, aktarılıyor...", commits.len());

    let mut imported = 0;
    let mut last_commit_hash = String::new();

    for line in &commits {
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() < 4 {
            continue;
        }

        let git_hash = parts[0];
        let author = parts[1];
        let timestamp: u64 = parts[2].parse().unwrap_or(0);
        let message = parts[3];

        // 2. Bu commit'in dosyalarını git ls-tree ile al
        let tree_output = Command::new("git")
            .args(["ls-tree", "-r", git_hash])
            .current_dir(git_repo_path)
            .output()?;

        let tree_text = String::from_utf8_lossy(&tree_output.stdout);
        let mut tree_entries = Vec::new();

        for tree_line in tree_text.lines() {
            // format: <mode> <type> <hash>\t<path>
            let parts: Vec<&str> = tree_line.splitn(2, '\t').collect();
            if parts.len() < 2 {
                continue;
            }
            let path = parts[1];
            let meta_parts: Vec<&str> = parts[0].split_whitespace().collect();
            if meta_parts.len() < 3 || meta_parts[1] != "blob" {
                continue;
            }
            let blob_hash = meta_parts[2];

            // 3. Blob içeriğini git cat-file ile al
            let blob_output = Command::new("git")
                .args(["cat-file", "-p", blob_hash])
                .current_dir(git_repo_path)
                .output()?;

            if blob_output.status.success() {
                let content = &blob_output.stdout;
                let hey_hash = hash_data(content);

                // KV store'a yaz
                let compressed = crate::storage::compression::compress(content)?;
                store.put(hey_hash.as_bytes(), &compressed)?;

                tree_entries.push(crate::core::object_model::TreeEntry {
                    name: path.to_string(),
                    hash: hey_hash,
                    is_dir: false,
                    is_chunked: false,
                    chunks: None,
                });
            }
        }

        // 4. Tree nesnesini oluştur ve kaydet
        let tree = crate::core::object_model::Tree { entries: tree_entries };
        let tree_json = serde_json::to_vec(&tree)?;
        let tree_hash = hash_data(&tree_json);
        store.put(tree_hash.as_bytes(), &tree_json)?;

        // 5. Commit nesnesini oluştur
        let parent_id = if last_commit_hash.is_empty() {
            None
        } else {
            Some(last_commit_hash.clone())
        };

        let commit = crate::core::object_model::Commit {
            parent_id,
            tree_hash: tree_hash.clone(),
            author: author.to_string(),
            timestamp,
            message: message.to_string(),
        };

        let commit_json = serde_json::to_vec(&commit)?;
        let commit_hash = hash_data(&commit_json);
        store.put(commit_hash.as_bytes(), &commit_json)?;

        last_commit_hash = commit_hash;
        imported += 1;
    }

    // 6. HEAD'i son commit'e ayarla
    if !last_commit_hash.is_empty() {
        let refs_dir = format!("{}/refs/heads", ctx.repo_path);
        fs::create_dir_all(&refs_dir)?;
        fs::write(format!("{}/main", refs_dir), &last_commit_hash)?;
        fs::write(format!("{}/HEAD", ctx.repo_path), "ref: refs/heads/main")?;
    }

    println!("Migration tamamlandı: {} commit aktarıldı.", imported);
    Ok(())
}
