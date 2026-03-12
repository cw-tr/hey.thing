use fastcdc::v2020::FastCDC;

pub struct Chunker;

impl Chunker {
    pub fn chunk_data(data: &[u8]) -> Vec<(String, Vec<u8>)> {
        // fastcdc v3.x limits: min_size must be reasonably small
        // Let's use 64KB, 128KB, 256KB for testing stability
        let avg_size = 128 * 1024; 
        let min_size = 64 * 1024;
        let max_size = 256 * 1024;

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
