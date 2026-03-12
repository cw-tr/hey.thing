use anyhow::Result;
use crate::core::verb_plugin::{VerbPlugin, ThingContext};

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

    fn run(&self, _ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.len() >= 2 && args[0] == "--from-git" {
            println!("Git migration süreci başlatıldı (Faz 2 Prototype)...");
            println!("UYARI: Bu sürümde yalnızca temel commit geçmişi aktarılır.");
            // TODO: Git repo tarama ve nesneleri KV store'a taşıma
            println!("Migration başarıyla tamamlandı (Benzetim).");
            return Ok(());
        }

        println!("Kullanım: hey import --from-git <git-repo-yolu>");
        Ok(())
    }
}
