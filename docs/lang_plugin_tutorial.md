# 🧩 hey.thing LangPlugin Geliştirme Rehberi

`hey.thing`, dosyaları birleştirirken sadece satır bazlı değil, kodun yapısını (fonksiyonlar, sınıflar vb.) anlayarak birleştirme yapabilen gelişmiş bir altyapıya sahiptir. Bu doküman, sisteme yeni diller için "akıllı birleştirme" (semantic merge) yeteneği katan `.thing` eklentilerinin nasıl geliştirileceğini anlatır.

---

## 🧐 LangPlugin Nedir?

`hey sync` veya `hey get` sırasında iki farklı koldaki (branch) değişiklikler çakıştığında (conflict), `hey` otomatik olarak bir eklenti arar. Örneğin `.rs` uzantılı bir dosya için `rs.thing` eklentisi yüklüyse, birleştirme görevini ona devreder.

Eklentiler **WebAssembly (WASM)** tabanlıdır. Bu sayede:
- **Dil Bağımsızlığı:** Eklentinizi Rust, C++, Go, AssemblyScript veya WASM'a derlenebilen herhangi bir dille yazabilirsiniz.
- **Güvenlik:** Eklenti izole bir sandbox içinde çalışır; dosya sistemine veya ağa erişemez.
- **Performans:** Yerel hıza yakın çalışır.

---

## 🛠️ Eklenti Arayüzü (ABI)

Her `.thing` eklentisi, host (`hey`) ile iletişim kurmak için belirli fonksiyonları dışa (export) aktarmalıdır.

### Gerekli Dış Fonksiyonlar

1.  **`allocate(size: usize) -> *mut u8`**: Host'un eklenti belleğinde yer ayırmasını sağlar (DOSYA içeriğini göndermek için).
2.  **`deallocate(ptr: *mut u8, size: usize)`**: Ayrılan belleği geri bırakır.
3.  **`merge(base_ptr: *mut u8, base_len: usize, local_ptr: *mut u8, local_len: usize, remote_ptr: *mut u8, remote_len: usize) -> i32`**: 
    - Asıl mantığın döndüğü yerdir. 
    - Dönüş değeri `0` ise başarılı, diğer değerler hatadır.
4.  **`get_result_ptr() -> *mut u8`**: Birleştirilmiş kodun bellekteki başlangıç noktasını döndürür.
5.  **`get_result_len() -> usize`**: Birleştirilmiş kodun uzunluğunu döndürür.

---

## 🚀 Örnek: Rust ile Eklenti Geliştirme

En kolay başlangıç yolu Rust kullanmaktır.

### 1. Cargo Kurulumu

Yeni bir kütüphane projesi başlatın:

```bash
cargo new --lib my-lang-plugin
cd my-lang-plugin
```

`Cargo.toml` dosyanızı şu şekilde güncelleyin:

```toml
[package]
name = "my-lang-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"] # WASM için zorunlu

[dependencies]
# Dilinizi parse etmek için tree-sitter, syn vb. ekleyebilirsiniz.
```

### 2. Kod Taslağı (`src/lib.rs`)

```rust
use std::mem;

static mut RESULT_PTR: *mut u8 = std::ptr::null_mut();
static mut RESULT_LEN: usize = 0;

#[no_mangle]
pub extern "C" fn allocate(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    unsafe { let _ = Vec::from_raw_parts(ptr, 0, size); }
}

#[no_mangle]
pub extern "C" fn get_result_ptr() -> *mut u8 { unsafe { RESULT_PTR } }

#[no_mangle]
pub extern "C" fn get_result_len() -> usize { unsafe { RESULT_LEN } }

#[no_mangle]
pub extern "C" fn merge(
    base_ptr: *mut u8, b_len: usize,
    local_ptr: *mut u8, l_len: usize,
    remote_ptr: *mut u8, r_len: usize
) -> i32 {
    let base = unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(base_ptr, b_len)) };
    let local = unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(local_ptr, l_len)) };
    let remote = unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(remote_ptr, r_len)) };

    // --- BURADA DİLİNİZE ÖZGÜ SEMANTİK MERGE MANTIĞINI ÇALIŞTIRIN ---
    // Örnek: Sadece local içeriğini geri döndürelim (fallback)
    let result = local.to_string(); 
    
    let mut result_bytes = result.into_bytes();
    unsafe {
        RESULT_PTR = result_bytes.as_mut_ptr();
        RESULT_LEN = result_bytes.len();
        mem::forget(result_bytes); // Belleğin otomatik silinmesini engelle
    }
    0 // Başarılı
}
```

---

## 🏗️ Derleme ve Yükleme

Eklentiyi WebAssembly (WASI) hedefi için derleyin:

```bash
rustup target add wasm32-wasi
cargo build --target wasm32-wasi --release
```

Oluşan `.wasm` dosyasını `hey`e eklemek için:

```bash
# Eklentiyi isimlendirerek kopyalayın
cp target/wasm32-wasi/release/my_lang_plugin.wasm ./js.thing

# hey'e tanıtın
hey lang add ./js.thing
```

Artık `hey`, `.js` uzantılı dosyalarda çatışma çıktığında sizin eklentinizi çağıracaktır.

---

## 💡 İpucu: Semantik Merge Nedir?

Basit birleştiriciler satırlara bakar. Akıllı birleştiriciler (LangPlugin) ise:
1.  Kodu AST (Abstract Syntax Tree) yapısına çevirir.
2.  Fonksiyonları ve yapıları isimlendirir.
3.  Eğer Local'de `A` fonksiyonu değişmiş, Remote'da `B` fonksiyonu eklenmişse; her ikisini de güvenle birleştirir. Satırların yer değiştirmesi semantik merger'ı yanıltmaz!

---
*Daha fazla örnek için [rs-thing](https://github.com/cw-tr/hey.thing/tree/main/plugins/rs-thing) referans eklentisini inceleyebilirsiniz.*
