use crate::core::verb_plugin::{ThingContext, VerbPlugin};
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct BranchVerb;

impl BranchVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for BranchVerb {
    fn name(&self) -> &str {
        "branch"
    }

    fn aliases(&self) -> &[&str] {
        &["br"]
    }

    fn help(&self) -> &str {
        "Dalları listeler veya yeni bir dal oluşturur"
    }

    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()> {
        let refs_dir = format!("{}/refs/heads", ctx.repo_path);
        fs::create_dir_all(&refs_dir)?;

        if args.is_empty() {
            // Dalları listele
            let current_branch =
                fs::read_to_string(format!("{}/HEAD", ctx.repo_path)).unwrap_or_default();

            for entry in fs::read_dir(&refs_dir)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let is_current = if current_branch.starts_with("ref: refs/heads/") {
                    current_branch.trim().ends_with(&name)
                } else {
                    false
                };

                if is_current {
                    println!("* {}", name);
                } else {
                    println!("  {}", name);
                }
            }
            return Ok(());
        }

        if args.len() >= 2 && args[0] == "new" {
            let branch_name = &args[1];
            let branch_file = format!("{}/{}", refs_dir, branch_name);

            if Path::new(&branch_file).exists() {
                return Err(anyhow!("'{}' adında bir dal zaten mevcut.", branch_name));
            }

            // Mevcut HEAD commit'ini al
            let head_content = fs::read_to_string(format!("{}/HEAD", ctx.repo_path))?;
            let current_hash = if head_content.starts_with("ref: ") {
                let ref_path = head_content.trim_start_matches("ref: ").trim();
                fs::read_to_string(format!("{}/{}", ctx.repo_path, ref_path))?
            } else {
                head_content.trim().to_string()
            };

            // Yeni dalı oluştur
            fs::write(&branch_file, current_hash)?;

            // Yeni dala geç (shift)
            fs::write(
                format!("{}/HEAD", ctx.repo_path),
                format!("ref: refs/heads/{}", branch_name),
            )?;

            println!("'{}' dalı oluşturuldu ve geçiş yapıldı.", branch_name);
            return Ok(());
        }

        println!("Kullanım:");
        println!("  hey branch          - Dalları listeler");
        println!("  hey branch new <ad> - Yeni dal oluşturur");

        Ok(())
    }
}
