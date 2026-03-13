use crate::core::ast_plugin::LangPlugin;
use anyhow::{Result, anyhow};
use std::path::Path;
use wasmtime::*;

pub struct WasmLangPlugin {
    name: String,
    extensions: Vec<String>,
    engine: Engine,
    module: Module,
}

impl WasmLangPlugin {
    /// Yeni bir .thing eklentisini diskten yükler ve Wasmtime modülü olarak hazırlar
    pub fn new(path: &Path) -> Result<Self> {
        let engine = Engine::default();
        let wasm_bytes = std::fs::read(path).map_err(|e| anyhow!("WASM okunamadı: {}", e))?;
        
        let module = Module::from_binary(&engine, &wasm_bytes)
            .map_err(|e| anyhow!("Geçersiz WASM binary: {}", e))?;
        
        let file_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        
        // İlk sürümde eklenti dosya isimleri özellik olarak alınabilir ('rs.thing' -> 'rs')
        // İleride wasm memory read ile eklentiden alınması sağlanacak.
        let ext = file_name.trim_end_matches(".thing").to_string();
        let extensions = vec![ext.clone()];
        
        Ok(Self {
            name: format!("{} AST Merger (.thing)", ext),
            extensions,
            engine,
            module,
        })
    }
}

impl LangPlugin for WasmLangPlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn extensions(&self) -> Vec<String> {
        self.extensions.clone()
    }
    
    fn merge(&self, _base: &str, _local: &str, _remote: &str) -> Result<String> {
        // Plugin izole sandbox ortamında başlatılır (her çağrıda sıfırdan oluşturmak güvenlik/state temizliği sağlar)
        let mut store = Store::new(&self.engine, ());
        let _instance = match Instance::new(&mut store, &self.module, &[]) {
            Ok(i) => i,
            Err(e) => return Err(anyhow!("Sandbox başlatılamadı: {}", e)),
        };
        
        // TODO: ABI (Application Binary Interface) Uygulaması
        // 1. instance'dan 'allocate' fonksiyonunu çağır (base + local + remote boyutları kadar)
        // 2. Wasm linear memory'e bu string'leri kopyala
        // 3. 'merge_ast' export fonksiyonunu işaretçilerle çağır
        // 4. Sonuç pointer'ını oku ve memory'den rust string'ine çevir.
        // 5. 'deallocate' çağır.
        
        Err(anyhow!("WASM ABI for ast_merge not yet implemented. Bailing out to standard text merge."))
    }
}
