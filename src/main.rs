use hey_thing::cmd;

use hey_thing::core::verb_plugin::ThingContext;
use hey_thing::plugins::verb_registry::VerbRegistry;
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
    registry.register(Box::new(cmd::shift::ShiftVerb::new()));
    registry.register(Box::new(cmd::undo::UndoVerb::new()));
    registry.register(Box::new(cmd::rewind::RewindVerb::new()));
    registry.register(Box::new(cmd::import::ImportVerb::new()));
    registry.register(Box::new(cmd::setup::SetupVerb::new()));
    registry.register(Box::new(cmd::sync::SyncVerb::new()));
    registry.register(Box::new(cmd::get::GetVerb::new()));
    registry.register(Box::new(cmd::lang::LangVerb::new()));
    registry.register(Box::new(cmd::verb::VerbVerb::new()));
    registry.register(Box::new(cmd::lock::LockVerb));
    registry.register(Box::new(cmd::lock::UnlockVerb));
    registry.register(Box::new(cmd::verify::VerifyVerb));
    registry.register(Box::new(cmd::sweep::SweepVerb));
    registry.register(Box::new(cmd::hydrate::HydrateVerb::new()));
    registry.register(Box::new(cmd::merge::MergeVerb::new()));

    // Eklentileri yükle
    let verb_paths = hey_thing::plugins::get_plugin_search_paths("verbs");
    registry.load_plugins_from_dirs(&verb_paths);

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

