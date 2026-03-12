use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::core::journal::Journal;
use std::fs;

pub struct RewindVerb;

impl RewindVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for RewindVerb {
    fn name(&self) -> &str {
        "rewind"
    }

    fn aliases(&self) -> &[&str] {
        &["go-to"]
    }

    fn help(&self) -> &str {
        "Zamanı geri sarar (ID veya zaman bazlı)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let target = args.first().ok_or_else(|| anyhow!("Lütfen hedef commit ID veya zaman (örn: '1 hour ago') belirtin."))?;
        
        let mut target_commit_hash = target.clone();

        // Zaman bazlı arama
        if target.contains("ago") || target.contains("hour") {
            let entries = Journal::read_all()?;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
            
            // Çok basit zaman parse (örn: "1" -> 3600 sn)
            let seconds_to_subtract: u64 = if target.contains("hour") {
                target.split_whitespace().next().unwrap_or("0").parse::<u64>().unwrap_or(0) * 3600
            } else {
                0
            };

            let target_time = now - seconds_to_subtract;
            
            if let Some(entry) = entries.iter().rev()
                .find(|e| e.timestamp <= target_time && e.action == "save") {
                target_commit_hash = entry.details["commit_hash"].as_str().unwrap_or_default().to_string();
            } else {
                return Err(anyhow!("Belirtilen zamanda uygun bir kayıt bulunamadı."));
            }
        }
        
        let store = ctx.store.as_ref().ok_or_else(|| anyhow!("Repo başlatılmamış."))?;
        if store.get(target_commit_hash.as_bytes())?.is_none() {
            return Err(anyhow!("Commit bulunamadı: {}", target_commit_hash));
        }

        let head_path = format!("{}/HEAD", ctx.repo_path);
        let head_content = fs::read_to_string(&head_path)?;
        
        if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ").trim();
            fs::write(format!("{}/{}", ctx.repo_path, ref_path), &target_commit_hash)?;
        } else {
            fs::write(head_path, &target_commit_hash)?;
        }

        println!("Zaman geri sarıldı. Hedef: {}", target_commit_hash);
        Ok(())
    }
}
