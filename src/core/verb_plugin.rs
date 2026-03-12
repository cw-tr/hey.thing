use crate::core::config::Config;
use crate::storage::kv_store::KvStore;
use anyhow::Result;
use std::path::Path;

pub struct ThingContext {
    pub repo_path: String,
    pub store: Option<KvStore>,
    pub config: Option<Config>,
}

impl ThingContext {
    pub fn load() -> Result<Self> {
        let repo_path = ".something".to_string();
        let config_path = ".configthing";

        let store = if Path::new(&repo_path).exists() {
            Some(KvStore::open(format!("{}/db", repo_path))?)
        } else {
            None
        };

        let config = if Path::new(config_path).exists() {
            Some(Config::load(config_path)?)
        } else {
            None
        };

        Ok(Self {
            repo_path,
            store,
            config,
        })
    }
}

pub trait VerbPlugin {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str];
    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()>;
    fn help(&self) -> &str;
}
