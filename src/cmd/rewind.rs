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

    fn help(&self) -> &str {
        "Zamanı geri sarar (ID veya zaman bazlı)"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let target = args.first().ok_or_else(|| anyhow!("Lütfen hedef commit ID veya zaman (örn: '1 hour ago') belirtin."))?;

        // --archived flag kontrolü
        let search_archived = args.iter().any(|a| a == "--archived");

        let mut target_commit_hash = target.clone();

        // Zaman bazlı arama
        if target.contains("ago") || target.contains("hour") || target.contains("minute") || target.contains("day") {
            // Hem aktif hem arşiv journal kayıtlarını birleştir
            let mut entries = Journal::read_all()?;
            if search_archived {
                let archived = Journal::read_all_archived()?;
                entries.extend(archived);
                entries.sort_by_key(|e| e.timestamp);
            }

            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();

            let seconds_to_subtract: u64 = parse_time_expression(target)?;
            let target_time = now.saturating_sub(seconds_to_subtract);

            if let Some(entry) = entries.iter().rev()
                .find(|e| e.timestamp <= target_time && e.action == "save") {
                target_commit_hash = entry.details["commit_hash"].as_str().unwrap_or_default().to_string();
            } else {
                return Err(anyhow!("Belirtilen zamanda uygun bir kayıt bulunamadı."));
            }
        } else if target == "--archived" {
            return Err(anyhow!("Lütfen hedef commit ID veya zaman ifadesi belirtin."));
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

/// Zaman ifadesini saniyeye çevirir.
/// Desteklenen formatlar: "1h", "30m", "2d", "1 hour ago", "30 minutes ago", "2 days ago"
fn parse_time_expression(expr: &str) -> Result<u64> {
    let expr = expr.trim().to_lowercase();

    // Kısa format: 1h, 30m, 2d
    if let Some(num_str) = expr.strip_suffix('h') {
        if let Ok(n) = num_str.trim().parse::<u64>() {
            return Ok(n * 3600);
        }
    }
    if let Some(num_str) = expr.strip_suffix('m') {
        if let Ok(n) = num_str.trim().parse::<u64>() {
            return Ok(n * 60);
        }
    }
    if let Some(num_str) = expr.strip_suffix('d') {
        if let Ok(n) = num_str.trim().parse::<u64>() {
            return Ok(n * 86400);
        }
    }

    // Uzun format: "1 hour ago", "30 minutes ago", "2 days ago"
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Ok(n) = parts[0].parse::<u64>() {
            let unit = parts[1];
            if unit.starts_with("hour") || unit.starts_with("saat") {
                return Ok(n * 3600);
            }
            if unit.starts_with("minute") || unit.starts_with("dakika") {
                return Ok(n * 60);
            }
            if unit.starts_with("day") || unit.starts_with("gün") || unit.starts_with("gun") {
                return Ok(n * 86400);
            }
        }
    }

    // Fallback: doğrudan sayı ise saniye olarak al
    if let Ok(n) = expr.replace("ago", "").trim().parse::<u64>() {
        return Ok(n);
    }

    Err(anyhow!("Zaman ifadesi anlaşılamadı: '{}'", expr))
}
