use anyhow::{Result, anyhow};
use crate::core::verb_plugin::{VerbPlugin, ThingContext};
use crate::crypto::hash::hash_data;

pub struct VerifyVerb;

impl VerbPlugin for VerifyVerb {
    fn name(&self) -> &str {
        "verify"
    }

    fn help(&self) -> &str {
        "Depodaki tüm nesnelerin bütünlüğünü (hash doğruluğunu) kontrol eder"
    }

    fn run(&self, ctx: &ThingContext, _args: &[String]) -> Result<()> {
        let store = ctx.store.as_ref()
            .ok_or_else(|| anyhow!("Repo başlatılmamış."))?;

        println!("🔍 Depo bütünlüğü doğrulanıyor...");

        let mut total = 0;
        let mut corrupted = 0;

        for (k, v) in store.iter() {
            total += 1;
            let key_str = String::from_utf8_lossy(&k);
            
            // Eğer key bir hash ise (64 karakter), veriyi doğrula
            if key_str.len() == 64 {
                // Not: Veri sıkıştırılmış olmalı, ama biz hash'i ORİJİNAL veri üzerinden alıyoruz.
                // Bazı objeleri (blobları) ham verisiyle, bazılarını (commit/tree) bincode haliyle hashliyoruz.
                // Aslında şu anki mimaride objenin KV store'daki key'i, decompress edilmiş halinin hashidir.
                
                if let Ok(decompressed) = crate::storage::compression::decompress(&v) {
                    let calculated = hash_data(&decompressed);
                    if calculated != key_str {
                        println!("  [!] Bozulmuş Nesne: {}", key_str);
                        corrupted += 1;
                    }
                } else {
                     // Sıkıştırma bozuk
                     println!("  [!] Sıkıştırma Hatası: {}", key_str);
                     corrupted += 1;
                }
            }
        }

        println!("\n--- Doğrulama Tamamlandı ---");
        println!("Toplam Nesne: {}", total);
        if corrupted == 0 {
            println!("✅ Durum: Mükemmel. Tüm nesneler sağlıklı.");
        } else {
            println!("❌ Durum: KRİTİK. {} adet bozuk nesne tespit edildi!", corrupted);
        }

        Ok(())
    }
}
