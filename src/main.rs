mod cmd;
mod core;
mod crypto;
mod i18n;
mod plugins;
mod storage;
#[cfg(test)]
mod tests;

use crate::core::verb_plugin::ThingContext;
use crate::plugins::verb_registry::VerbRegistry;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("hey.thing - Kullanım: hey <komut> [seçenekler]");
        return;
    }

    let command = &args[1];

    // Context'i yükle
    let context = match ThingContext::load() {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Zaman uyumsuzluğu: {}", e);
            return;
        }
    };

    let mut registry = VerbRegistry::new();

    // Built-in verb'lerin kaydedilmesi
    registry.register(Box::new(cmd::init::InitVerb::new()));
    registry.register(Box::new(cmd::save::SaveVerb::new()));
    registry.register(Box::new(cmd::show::ShowVerb::new()));

    match registry.find(command) {
        Some(verb) => {
            if let Err(e) = verb.run(&context, &args[2..]) {
                eprintln!("Hata: {}", e);
            }
        }
        None => {
            println!("Bilinmeyen komut: {}", command);
            registry.list_help();
        }
    }
}
