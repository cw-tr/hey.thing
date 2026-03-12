use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JournalEntry {
    pub timestamp: u64,
    pub action: String,
    pub details: serde_json::Value,
}

pub struct Journal;

impl Journal {
    pub fn log(action: &str, details: serde_json::Value) -> Result<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        let entry = JournalEntry {
            timestamp,
            action: action.to_string(),
            details,
        };

        let json = serde_json::to_string(&entry)?;

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(".something/journal")?;

        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn read_all() -> Result<Vec<JournalEntry>> {
        let path = ".something/journal";
        Self::read_from_file(path)
    }

    pub fn read_from_file(path: &str) -> Result<Vec<JournalEntry>> {
        if !Path::new(path).exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)?;
        let mut entries = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                let entry: JournalEntry = serde_json::from_str(line)?;
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// Journal pruning: retention_days gün öncesindeki kayıtları arşive taşır.
    /// Arşiv: .something/archive/journal_<timestamp> dosyasına yazılır.
    pub fn prune(retention_days: u64) -> Result<PruneResult> {
        let entries = Self::read_all()?;
        if entries.is_empty() {
            return Ok(PruneResult { archived: 0, remaining: 0 });
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let cutoff = now.saturating_sub(retention_days * 86400);

        let mut old_entries = Vec::new();
        let mut fresh_entries = Vec::new();

        for entry in entries {
            if entry.timestamp < cutoff {
                old_entries.push(entry);
            } else {
                fresh_entries.push(entry);
            }
        }

        if old_entries.is_empty() {
            return Ok(PruneResult {
                archived: 0,
                remaining: fresh_entries.len(),
            });
        }

        // Arşiv dizinini oluştur
        let archive_dir = ".something/archive";
        fs::create_dir_all(archive_dir)?;

        // Eski kayıtları arşiv dosyasına yaz
        let archive_file = format!("{}/journal_{}", archive_dir, now);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&archive_file)?;

        for entry in &old_entries {
            let json = serde_json::to_string(entry)?;
            writeln!(file, "{}", json)?;
        }

        // Ana journal dosyasını sadece taze kayıtlarla yeniden yaz
        let journal_path = ".something/journal";
        let mut journal_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(journal_path)?;

        for entry in &fresh_entries {
            let json = serde_json::to_string(entry)?;
            writeln!(journal_file, "{}", json)?;
        }

        println!("{} eski kayıt arşivlendi → {}", old_entries.len(), archive_file);

        Ok(PruneResult {
            archived: old_entries.len(),
            remaining: fresh_entries.len(),
        })
    }

    /// Arşiv dosyalarını listele
    pub fn list_archives() -> Result<Vec<String>> {
        let archive_dir = ".something/archive";
        if !Path::new(archive_dir).exists() {
            return Ok(Vec::new());
        }

        let mut archives = Vec::new();
        for entry in fs::read_dir(archive_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("journal_") {
                archives.push(entry.path().to_string_lossy().to_string());
            }
        }
        archives.sort();
        Ok(archives)
    }

    /// Arşivlenmiş kayıtlardan belirli bir commit hash'i ara
    pub fn find_in_archives(commit_hash: &str) -> Result<Option<JournalEntry>> {
        let archives = Self::list_archives()?;
        for archive_path in archives {
            let entries = Self::read_from_file(&archive_path)?;
            for entry in entries {
                if let Some(hash) = entry.details.get("commit_hash").and_then(|v| v.as_str()) {
                    if hash == commit_hash {
                        return Ok(Some(entry));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Tüm arşivlerdeki kayıtları oku (rewind --archived için)
    pub fn read_all_archived() -> Result<Vec<JournalEntry>> {
        let archives = Self::list_archives()?;
        let mut all = Vec::new();
        for archive_path in archives {
            let entries = Self::read_from_file(&archive_path)?;
            all.extend(entries);
        }
        Ok(all)
    }
}

pub struct PruneResult {
    pub archived: usize,
    pub remaining: usize,
}
