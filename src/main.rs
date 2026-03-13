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

    // Eklentileri yükle (~/.something/verbs/)
    let something_dir = hey_thing::plugins::get_something_dir();
    registry.load_plugins_from_dir(&something_dir.join("verbs"));

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

#[cfg(test)]
mod manual_inspect {
    use super::*;
    #[test]
    fn test_inspect_tree() {
        let store = crate::storage::kv_store::KvStore::open(".something/db").unwrap();
        let tree_hash = "56851dcff88db93a8247b86af9ca0371ec777d3c424ad9cee8481aea9cd8bf2b";
        let data = store.get(tree_hash.as_bytes()).unwrap().unwrap();
        println!("DATA_START:{}:DATA_END", String::from_utf8_lossy(&data));
    }
}
