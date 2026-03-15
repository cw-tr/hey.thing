use crate::core::ast_plugin::LangPlugin;
use anyhow::{Result, anyhow};
use std::path::Path;
use wasmtime::*;

use wasmtime_wasi::p1::{WasiP1Ctx, add_to_linker_sync};
use wasmtime_wasi::WasiCtxBuilder;

struct HostState {
    wasi: WasiP1Ctx,
}

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
    
    fn merge(&self, base: &str, local: &str, remote: &str) -> Result<crate::core::ast_plugin::MergeResult> {
        let wasi = WasiCtxBuilder::new().inherit_stdout().inherit_stderr().build_p1();
        let mut store = Store::new(&self.engine, HostState { wasi });
        
        let mut linker = Linker::new(&self.engine);
        add_to_linker_sync(&mut linker, |s: &mut HostState| &mut s.wasi)
            .map_err(|e| anyhow!("WASI Linker hatası: {}", e))?;

        let instance = linker.instantiate(&mut store, &self.module)
            .map_err(|e| anyhow!("Sandbox başlatılamadı: {}", e))?;

        let allocate = instance.get_typed_func::<u32, u32>(&mut store, "allocate")?;
        let deallocate = instance.get_typed_func::<(u32, u32), ()>(&mut store, "deallocate")?;
        let merge = instance.get_typed_func::<(u32, u32, u32, u32, u32, u32), i32>(&mut store, "merge")?;
        let get_result_ptr = instance.get_typed_func::<(), u32>(&mut store, "get_result_ptr")?;
        let get_result_len = instance.get_typed_func::<(), u32>(&mut store, "get_result_len")?;

        let memory = instance.get_memory(&mut store, "memory").ok_or_else(|| anyhow!("Memory not found"))?;

        // 1. Memory hazırlığı
        let base_ptr = allocate.call(&mut store, base.len() as u32)?;
        memory.write(&mut store, base_ptr as usize, base.as_bytes())?;

        let local_ptr = allocate.call(&mut store, local.len() as u32)?;
        memory.write(&mut store, local_ptr as usize, local.as_bytes())?;

        let remote_ptr = allocate.call(&mut store, remote.len() as u32)?;
        memory.write(&mut store, remote_ptr as usize, remote.as_bytes())?;

        // 2. Birleştirme çağrısı
        let status = merge.call(&mut store, (
            base_ptr, base.len() as u32,
            local_ptr, local.len() as u32,
            remote_ptr, remote.len() as u32
        ))?;

        // status < 0 -> Hata
        if status < 0 {
            return Err(anyhow!("Plugin merge failed with status {}", status));
        }

        // 3. Sonucu al
        let result_ptr = get_result_ptr.call(&mut store, ())?;
        let result_len = get_result_len.call(&mut store, ())?;

        let mut result_buf = vec![0u8; result_len as usize];
        memory.read(&mut store, result_ptr as usize, &mut result_buf)?;
        let result = String::from_utf8(result_buf)?;

        // 4. Temizlik
        let _ = deallocate.call(&mut store, (base_ptr, base.len() as u32));
        let _ = deallocate.call(&mut store, (local_ptr, local.len() as u32));
        let _ = deallocate.call(&mut store, (remote_ptr, remote.len() as u32));

        Ok(crate::core::ast_plugin::MergeResult {
            content: result,
            has_conflict: status == 1,
        })
    }
}
