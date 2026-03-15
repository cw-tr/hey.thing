use anyhow::Result;
use sled::Db;
use std::path::Path;

pub struct KvStore {
    db: Db,
}

impl KvStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = sled::Config::new()
            .path(path)
            .cache_capacity(1_000_000_000) // 1GB Cache sınırı (12GB RAM için ideal)
            .flush_every_ms(Some(1000))    // I/O yoğunluğunu azaltmak için 1sn'de bir flush
            .mode(sled::Mode::LowSpace);   // Disk şişmesini engellemek için agresif sıkıştırma
        
        let db = config.open()?;
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

    pub fn insert_batch(&self, items: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let mut batch = sled::Batch::default();
        for (k, v) in items {
            batch.insert(k, v);
        }
        self.db.apply_batch(batch)?;
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> {
        self.db.iter().filter_map(|res| res.ok().map(|(k, v)| (k.to_vec(), v.to_vec())))
    }

    pub fn remove(&self, key: &[u8]) -> Result<()> {
        self.db.remove(key)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }
}
