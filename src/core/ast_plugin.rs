use anyhow::Result;

/// A Language Plugin responsible for smart AST-aware or custom merging.
pub trait LangPlugin: Send + Sync {
    /// Eklenti adı (örn: "Rust AST Merger")
    fn name(&self) -> &str;
    
    /// Desteklenen dosya uzantıları (örn: ["rs", "rlib"])
    fn extensions(&self) -> Vec<String>;
    
    /// 3-way merge işlemi
    /// Başarılıysa Ok(String) döndürür, Başarısız/Emin değilse Err(...) (fallback'e düşmesi için)
    fn merge(&self, base: &str, local: &str, remote: &str) -> Result<String>;
}
