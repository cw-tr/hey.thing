use anyhow::Result;

pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let compressed = zstd::encode_all(data, 3)?;
    Ok(compressed)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let decompressed = zstd::decode_all(data)?;
    Ok(decompressed)
}
