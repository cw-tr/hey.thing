use crate::core::journal::Journal;
use crate::core::object_model::Commit;
use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::{Result, anyhow};
use std::fs;

pub struct UndoVerb;

impl UndoVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for UndoVerb {
    fn name(&self) -> &str {
        "undo"
    }

    fn aliases(&self) -> &[&str] {
        &["revert-last"]
    }

    fn help(&self) -> &str {
        "Son işlemi geri alır"
    }

    fn run(&self, ctx: &ThingContext, _args: &[String]) -> Result<()> {
        let entries = Journal::read_all()?;
        let last_save = entries
            .iter()
            .rev()
            .find(|e| e.action == "save")
            .ok_or_else(|| anyhow!("Geri alınacak bir kayıt işlemi bulunamadı."))?;

        let commit_hash = last_save.details["commit_hash"]
            .as_str()
            .ok_or_else(|| anyhow!("Journal verisi bozuk."))?;

        // Commit'i bul ve parent'ına geç
        let store = ctx
            .store
            .as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış."))?;
        let commit_data = store
            .get(commit_hash.as_bytes())?
            .ok_or_else(|| anyhow!("Commit bulunamadı: {}", commit_hash))?;
        let commit: Commit = serde_json::from_slice(&commit_data)?;

        if let Some(parent_id) = commit.parent_id {
            // Shift verb'ine benzer mantıkla parent'a geç
            // Şimdilik sadece HEAD'i güncelleyelim, tam undo mekanizması Phase 2 sonunda tamamlanacak.
            let head_path = format!("{}/HEAD", ctx.repo_path);
            let head_content = fs::read_to_string(&head_path)?;

            if head_content.starts_with("ref: ") {
                let ref_path = head_content.trim_start_matches("ref: ").trim();
                fs::write(format!("{}/{}", ctx.repo_path, ref_path), &parent_id)?;
            } else {
                fs::write(head_path, &parent_id)?;
            }

            println!("İşlem geri alındı. Yeni HEAD: {}", parent_id);
        } else {
            return Err(anyhow!("Bu ilk commit, öncesi yok."));
        }

        Ok(())
    }
}
