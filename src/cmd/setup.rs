use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::core::config::Config;
use std::io::{self, Write};

pub struct SetupVerb;

impl SetupVerb {
    pub fn new() -> Self {
        Self
    }
}

impl VerbPlugin for SetupVerb {
    fn name(&self) -> &str {
        "setup"
    }

    fn help(&self) -> &str {
        "Kurulum ve güvenlik yapılandırma ayarlarını yönetir (örn: setup trust)"
    }

    fn run(&self, _ctx: &ThingContext, args: &[String]) -> Result<()> {
        if args.len() < 1 {
            return Err(anyhow!("Kullanım: hey setup <komut>"));
        }

        match args[0].as_str() {
            "trust" => {
                println!("UYARI: Bu işlem çalışma dizinindeki .configthing ayarlarını (kısmen hook'ları)");
                println!("güvenilir olarak işaretleyecektir. Kaynağını bilmediğiniz bir kodu onaylamayın.");
                print!("Devam etmek istiyor musunuz? [e/H]: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim().to_lowercase() == "e" || input.trim().to_lowercase() == "y" {
                    Config::mark_trusted(".configthing")?;
                    println!(".configthing dosyası güvenilir olarak işaretlendi.");
                } else {
                    println!("İşlem iptal edildi.");
                }
            }
            _ => {
                return Err(anyhow!("Bilinmeyen setup alt komutu: {}", args[0]));
            }
        }

        Ok(())
    }
}
