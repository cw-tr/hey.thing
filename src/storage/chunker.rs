use fastcdc::v2020::FastCDC;
use anyhow::Result;

pub struct Chunker;

impl Chunker {
    pub fn chunk_data(data: &[u8]) -> Vec<(String, Vec<u8>)> {
        let avg_size = 4 * 1024 * 1024; // 4MB
        let min_size = 2 * 1024 * 1024; // 2MB
        let max_size = 8 * 1024 * 1024; // 8MB

        let chunker = FastCDC::new(data, min_size, avg_size, max_size);
        let mut chunks = Vec::new();

        for chunk in chunker {
            let chunk_data = &data[chunk.offset..chunk.offset + chunk.length];
            let hash = crate::crypto::hash::hash_data(chunk_data);
            chunks.push((hash, chunk_data.to_vec()));
        }

        chunks
    }
}
