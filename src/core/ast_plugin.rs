use anyhow::Result;

pub struct MergeResult {
    pub content: String,
    pub has_conflict: bool,
}

/// A Language Plugin responsible for smart AST-aware or custom merging.
pub trait LangPlugin: Send + Sync {
    /// Eklenti adı (örn: "Rust AST Merger")
    fn name(&self) -> &str;
    
    /// Desteklenen dosya uzantıları (örn: ["rs", "rlib"])
    fn extensions(&self) -> Vec<String>;
    
    /// 3-way merge işlemi
    /// Başarılıysa Ok(MergeResult) döndürür
    fn merge(&self, base: &str, local: &str, remote: &str) -> Result<MergeResult>;
}
