use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
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
        if !std::path::Path::new(path).exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(path)?;
        let mut entries = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                let entry: JournalEntry = serde_json::from_str(line)?;
                entries.push(entry);
            }
        }
        Ok(entries)
    }
}
