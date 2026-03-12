use anyhow::Result;
use sled::Db;
use std::path::Path;

pub struct KvStore {
    db: Db,
}

impl KvStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.insert(key, value)?;
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let res = self.db.get(key)?.map(|v| v.to_vec());
        Ok(res)
    }

    #[allow(dead_code)]
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
