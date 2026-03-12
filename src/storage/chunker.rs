use fastcdc::v2020::FastCDC;

/// Varsayılan LFS eşiği: 10MB
pub const DEFAULT_LFS_THRESHOLD: usize = 10 * 1024 * 1024;

/// Bilinen binary dosya magic byte'ları
const BINARY_MAGIC: &[&[u8]] = &[
    b"\x7fELF",       // ELF executable
    b"\x89PNG",        // PNG
    b"\xff\xd8\xff",   // JPEG
    b"GIF8",           // GIF
    b"PK\x03\x04",    // ZIP/JAR/DOCX
    b"PK\x05\x06",    // ZIP empty
    b"\x1f\x8b",      // gzip
    b"BM",             // BMP
    b"\x00\x00\x01\x00", // ICO
    b"RIFF",           // WAV/AVI
    b"\x1a\x45\xdf\xa3", // MKV/WebM
    b"\x00\x00\x00\x18ftypmp4",  // MP4 (kısa)
    b"\x00\x00\x00\x1cftyp",     // MP4/MOV
    b"MZ",             // Windows PE/EXE
    b"\xca\xfe\xba\xbe", // Mach-O fat binary
    b"\xcf\xfa\xed\xfe", // Mach-O 64-bit
    b"\xfe\xed\xfa\xce", // Mach-O 32-bit
    b"\xfd7zXZ\x00",  // XZ
    b"Rar!\x1a\x07",  // RAR
    b"\x28\xb5\x2f\xfd", // Zstandard
];

pub struct Chunker;

impl Chunker {
    pub fn chunk_data(data: &[u8]) -> Vec<(String, Vec<u8>)> {
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

/// Dosyanın binary olup olmadığını tespit eder.
/// 1. Magic bytes kontrolü
/// 2. Null byte kontrolü (ilk 8KB)
/// 3. Entropi analizi (yüksek entropi = sıkıştırılmış/şifrelenmiş = binary)
pub fn is_binary(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    // 1. Magic bytes kontrolü
    for magic in BINARY_MAGIC {
        if data.len() >= magic.len() && &data[..magic.len()] == *magic {
            return true;
        }
    }

    // 2. İlk 8KB'da null byte varsa binary
    let check_len = data.len().min(8192);
    if data[..check_len].contains(&0x00) {
        return true;
    }

    // 3. Entropi analizi (Shannon entropy)
    let entropy = calculate_entropy(&data[..check_len]);
    // Entropi > 7.0 genellikle sıkıştırılmış/şifrelenmiş veri
    if entropy > 7.0 {
        return true;
    }

    false
}

/// Dosya boyutunun LFS eşiğini aşıp aşmadığını kontrol eder
pub fn is_large_file(size: usize, threshold: usize) -> bool {
    size >= threshold
}

/// Shannon entropi hesaplama (0.0 - 8.0 arası)
fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut freq = [0u64; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}
