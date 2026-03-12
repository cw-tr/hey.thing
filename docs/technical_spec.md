# hey.thing — Teknik Tanıtım Belgesi
*Açık Kaynak Versiyon Kontrol Motoru*

---

## Tek Cümlede Ne?

**hey.thing**, Git'in 20 yıllık tasarım borçlarını sıfırdan çözen; modern kriptografi, semantik kod birleştirme ve akıllı CLI mimarisiyle tasarlanmış, tamamen açık kaynaklı bir Versiyon Kontrol Sistemi motorudur.

hey.thing, Somewhere ekosisteminin açık kaynak çekirdeğidir — ama bağımsız olarak da tam işlevsel bir VCS'dir.

---

## Neden Yeni Bir VCS?

Git 2005'te tek bir geliştirici için tasarlandı. O günden bu yana eklenen her özellik, üstüne yama olarak geldi. Sonuç: 20 yıllık tasarım borcuyla yüklü, öğrenmesi zor, binary dosyalarda yetersiz, güvenlik temeli çürük bir sistem.

**hey.thing bu borçları miras almıyor. Baştan tasarlıyor.**

---

## Temel Mimari Kararlar

### Dil: Rust
Memory safety, sıfır bağımlılık, cross-compile. Git'in C ile yaptığı hatayı yapmıyoruz.

### Depolama: KV Veritabanı (Sled → RocksDB)
Git'in `.git/objects/` içindeki yüz binlerce küçük dosya yaklaşımı yerine, tek bir optimize KV store. Klonlama ve okuma hızı dramatik artıyor. Sled geliştirme fazında, RocksDB production ölçeğinde.

### Hashing: BLAKE3
SHA-1'den 10x hızlı. Donanımsal paralellik. Collision-resistant. Merkle Tree dizilimiyle bütünlük doğrulaması.

### Config: TOML
YAML'ın girinti tuzakları yok. JSON'ın yorum satırı sorunu yok. `.configthing` dosyası proje kökünde, insan tarafından okunabilir.

### Veri Modeli: Event Sourcing
Her eylem append-only journal'a kaydedilir. Hiçbir şey gerçekten silinmez. "Reflog'u bul, komutu çalıştır" karmaşası yerine `hey undo`.

---

## `hey.thing` — CLI Mimarisi

Tek binary, tüm platformlarda aynı davranış. Symlink yok, alias yok, platform farkı yok.

```bash
hey save "kullanıcı girişi tamamlandı"
hey shift feature-login
hey sync
hey rewind "1 hour ago"
hey undo
hey show
hey init
hey get cw-tr/myproject
```

PATH'te `hey` komutu `hey.thing` binary'sine yönlendirilir. Geri kalan her şey binary'nin içinde.

**`VerbPlugin` Trait — Modüler Verb Mimarisi:**

`hey.thing` hardcoded `match` bloğu kullanmaz. Her verb bağımsız bir `*.thing` eklentisidir. Binary başlarken `~/.something/verbs/` dizinini tarar ve ne bulursa yükler.

```rust
// core/verb_plugin.rs — hey.thing'in bildiği tek şey bu
pub trait VerbPlugin {
    fn name(&self) -> &str;           // "save", "shift", "merge"
    fn aliases(&self) -> &[&str];     // ["s", "commit"] gibi kısayollar
    fn run(&self, ctx: &ThingContext, args: &[String]) -> Result<()>;
    fn help(&self) -> &str;           // hey help → açıklama satırı
}
```

**Verb kayıt defteri — startup'ta otomatik:**

```rust
// cmd/hey.rs
let registry = VerbRegistry::load("~/.something/verbs/");
// → save.thing, shift.thing, sync.thing... + kurulmuş topluluk verb'leri

match registry.find(args[0]) {
    Some(verb) => verb.run(&ctx, &args[1..]),
    None       => registry.help(),   // bilinmeyen verb → tüm liste
}
```

**Korumalı verb'ler — üzerine yazılamaz:**

```rust
const PROTECTED: &[&str] = &[
    "save", "shift", "sync", "undo", "rewind",
    "show", "init", "get", "branch", "import",
    "verb", "plugin", "help",
];
// Biri save.thing adında kötü niyetli eklenti yayınlasa bile core ezilmez.
```

**Neden bu mimari:**
- Hardcoded `match` yok — hey.thing verb'leri bilmek zorunda değil
- İsteyen `merge.thing`, `deploy.thing`, `release.thing` yazıp ekleyebilir
- Kurumsal özel verb: `hey deploy staging` → şirketin kendi `deploy.thing`'i
- `hey help` otomatik dolar — eklenen her verb listeye girer
- LangPlugin ile tam simetri: diller nasıl modülerse, verb'ler de öyle

**Kurulum sonrası dizin yapısı:**
```
~/.something/
├── bin/
│   └── hey.thing            ← tek binary, PATH'e eklenir
├── verbs/                   ← verb eklentileri
│   ├── save.thing           ← built-in
│   ├── shift.thing          ← built-in
│   ├── sync.thing           ← built-in
│   ├── undo.thing           ← built-in
│   ├── rewind.thing         ← built-in
│   └── merge.thing          ← topluluk eklentisi — otomatik algılandı
├── langs/                   ← AST dil eklentileri
│   ├── rs.thing             ← built-in
│   └── py.thing             ← resmi
├── translation/             ← arayüz dil paketleri
│   ├── en.thing             ← built-in, fallback
│   └── tr.thing             ← resmi
├── hooks/                   ← Somewhere/Jira gibi entegrasyonlar
│   └── jira.thing
├── keys/
│   ├── id_thing_ed25519
│   └── id_thing_ed25519.pub
└── config/
    └── global.toml

~/proje/                     ← Proje dizini
├── .something/              ← VCS Veri ve Nesne Deposu (.git gibi)
├── .configthing             ← Proje ayarları (TOML)
├── src/
└── ...

Proje dizininde binary yok. `hey init` hem `.something` dizinini hem de `.configthing` dosyasını oluşturur.

---

## CLI Komut Seti

### Çalışma Alanı

| Komut | Açıklama | Git Karşılığı |
|---|---|---|
| `hey init` | KV veritabanı oluştur | `git init` |
| `hey get <ns>` | Projeyi indir | `git clone` |
| `hey import --from-git` | Git geçmişini aktar | — |

### Günlük İş Akışı

| Komut | Açıklama | Git Karşılığı |
|---|---|---|
| `hey save "mesaj"` | Değişiklikleri kaydet | `git add . && git commit -m` |
| `hey sync` | Bulutla akıllı eşitle | `git fetch && git rebase && git push` |
| `hey show` | Proje durumu | `git status` |

### Zaman Makinesi

| Komut | Açıklama | Git Karşılığı |
|---|---|---|
| `hey undo` | Son eylemi geri al | `git reset HEAD~1` (tehlikeli) |
| `hey rewind "2h ago"` | 2 saat öncesine dön | `git reflog` + `git reset` (karmaşık) |
| `hey rewind SaveID_A1B2` | ID'ye git | — |

### Dal Yönetimi

| Komut | Açıklama | Git Karşılığı |
|---|---|---|
| `hey shift <dal>` | Dal değiştir | `git switch` |
| `hey branch new <isim>` | Dal oluştur ve geç | `git checkout -b` |
| `hey branch list` | Dalları listele | `git branch -a` |

---

## `.configthing` — Bayrak Karmaşasının Sonu

Her seferinde `-a`, `--hard`, `--ours`, `--rebase` yazmak yok. Davranışlar config'de tanımlı:

```toml
[user]
name = "Mukan Erkin"
crypto_key = "~/.something/keys/id_thing_ed25519"

[behavior]
auto_stage_all = true          # hey save → otomatik add
ignore_empty_commits = true    # Değişiklik yoksa commit atma
journal_retention = "90d"      # Zaman makinesi derinliği

[hooks]
pre_save = "cargo fmt"         # Kaydetmeden önce formatla

[security]
trusted_config_hash = "sha256:abc123..."  # Hook güvenlik kilidi

[sync]
default_remote = "origin"
conflict_strategy = "tui"      # Conflict'te TUI aç
```

---

## Güvenlik Modeli

### BLAKE3 + Merkle Tree
Her nesne BLAKE3 ile hashlenir. Commit zinciri Merkle Tree ile doğrulanır. Geçmiş manipülasyonu tespit edilebilir.

### ed25519 Kimlik Doğrulama
`hey init` sırasında otomatik anahtar çifti oluşturulur. Git'te herhangi biri herhangi birinin adıyla commit atabilir — hey.thing'de her commit kriptografik olarak imzalıdır.

### Hook Güvenlik Modeli
Klonlanan bir repodaki `.configthing` hook'ları varsayılan olarak devre dışı. Kullanıcı `hey setup trust` ile bilinçli onay verir. Supply chain saldırısı vektörü kapatılmıştır.

---

## Semantik AST Merge — Modüler Dil Eklenti Sistemi

Git, iki değişikliği satır-satır karşılaştırır. Bir fonksiyon taşınmışsa conflict verir. hey.thing dili anlar — ama her dili kendisi bilmek zorunda değil.

### `LangPlugin` Trait Mimarisi

hey.thing içinde bir dil arayüzü (`trait`) tanımlıdır. Her dil eklentisi bu arayüzü implemente eden bağımsız bir WASM binary'sidir:

```rust
// core/ast_plugin.rs — hey.thing'in bildiği tek şey bu
pub trait LangPlugin {
    fn extensions(&self) -> &[&str];          // ["py", "pyw"]
    fn parse(&self, src: &[u8]) -> AstTree;
    fn diff(&self, old: &AstTree, new: &AstTree) -> SemanticDiff;
    fn merge(&self, base: &AstTree, ours: &AstTree, theirs: &AstTree) -> MergeResult;
}
```

hey.thing `auth.py` dosyasını gördüğünde `.py` uzantısını okur, registry'den `py.thing` eklentisini yükler, AST merge'i ona devreder. Dili kendisi bilmiyor — trait'i biliyor.

### Dil Eklentileri (`*.thing`)

```
~/.something/langs/
├── rs.thing    ← Rust (built-in, referans implementasyon, açık kaynak)
├── py.thing    ← Python  (resmi)
├── js.thing    ← JS/TS   (resmi)
├── go.thing    ← Go      (resmi)
├── sol.thing   ← Solidity        (topluluk)
├── lua.thing   ← Lua             (topluluk)
├── zig.thing   ← Zig             (topluluk)
└── cobol.thing ← COBOL           (kurumsal/private)
```

Her `*.thing` eklentisi:
- tree-sitter grammar + LangPlugin wrapper
- WASM formatında — güvenli sandbox (dosya sistemi erişimi yok)
- Platform bağımsız: Linux'ta derlenmiş `py.thing` Windows'ta da çalışır

### Eklenti Yönetimi

```bash
hey lang add py.thing          # resmi kayıt defterinden
hey lang add sol.thing         # topluluk eklentisi
hey lang add ./internal.thing  # yerel/kurumsal
hey lang list                  # yüklü eklentiler
hey lang test py.thing         # standart test suite çalıştır
```

### `.configthing` Entegrasyonu

```toml
[langs]
auto_detect = true              # uzantıya göre otomatik yükle
python = "py.thing@2.1.0"      # versiyon pinleme
javascript = "js.thing"         # latest
solidity = "sol.thing"          # topluluk eklentisi

[langs.fallback]
strategy = "line-diff"          # eklenti yoksa satır bazlı merge
```

### Ekosistem Katmanları

```
BUILT-IN                         hey.thing içinde, her zaman mevcut
────────────────────────         ──────────────────────────────────
rs.thing                         Rust — referans implementasyon

RESMİ EKLENTÎ                    Somewhere tarafından bakımı yapılır
─────────────────────            ──────────────────────────────────
py.thing  js.thing               Python, JavaScript/TypeScript
go.thing  rb.thing               Go, Ruby

TOPLULUK                         thing-langs registry (açık kaynak)
──────────────                   ──────────────────────────────────
sol.thing  lua.thing             Solidity, Lua
zig.thing  swift.thing           Zig, Swift
cpp.thing  java.thing            C++, Java
...                              Herhangi bir dil

KURUMSAL                         Private registry
───────────────                  ────────────────
cobol.thing                      Şirkete özel DSL'ler
sap-abap.thing                   İç sistemler
internal-dsl.thing
```

### thing-langs Registry

tree-sitter ekosisteminin hey.thing versiyonu. Her eklenti için:
- Standart test suite (`hey lang test`)
- Kalite rozeti: "verified by Somewhere team"
- İndirme istatistikleri
- Versiyon geçmişi

**Topluluk katkı bariyeri düşük:** tree-sitter grammar'ı zaten varsa `py.thing` gibi bir wrapper yazmak 1-2 günlük iş. `rs.thing` açık kaynak referans implementasyon olarak bu soruyu cevaplar: "Kendi dilin için nasıl `*.thing` yazarsın?"

---

## Native Büyük Dosya Desteği

Harici eklenti yok. hey.thing dosyayı tanır:

1. Magic bytes + entropi analizi → binary mi metin mi?
2. Binary dosya → FastCDC algoritmasıyla ~4MB chunk'lara böl
3. Chunk deduplication → aynı chunk farklı dosyalarda tekrar saklanmaz
4. Sıkıştırma → zstd ile optimal oran/hız

100GB Unity projesi, Git LFS operasyonel kabusu olmadan.

---

## Veri Modeli

```
Journal (append-only)
└── Event: SAVE {tree_hash, author, timestamp, message}
    └── Tree: {dosya_adı → blob_hash, boş_klasör_desteği}
        └── Blob: BLAKE3(içerik) → KV store
```

**Boş klasör desteği:** `.gitkeep` hilesi yok. Tree nesnesi klasörün kendisini (boş olsa bile) kaydeder.

**Event Sourcing:** Hiçbir şey gerçekten silinmez. Her save, shift, sync, delete — journal'da. `hey rewind` zaman makinesinin yakıtı budur.

---

## Proje Dizin Yapısı

```
hey-thing-src/
├── .configthing
├── Cargo.toml
├── cmd/                    # CLI katmanı
│   └── hey.rs              # hey.thing giriş noktası + VerbRegistry yükleme
├── core/                   # İş mantığı
│   ├── hey.rs              # ThingContext — proje durumu, config, KV bağlantısı
│   ├── config.rs           # .configthing parser
│   ├── object_model.rs     # Commit, Tree, Blob
│   ├── verb_plugin.rs      # VerbPlugin trait + VerbRegistry + PROTECTED listesi
│   ├── ast_plugin.rs       # LangPlugin trait tanımı
│   └── journal.rs          # Event sourcing
├── storage/                # Veri katmanı
│   ├── kv_store.rs         # Sled/RocksDB bağlayıcı
│   ├── vfs.rs              # Sanal dosya sistemi
│   ├── chunker.rs          # FastCDC
│   ├── compression.rs      # zstd/LZ4
│   └── offline_cache.rs    # Uçak modu
├── crypto/                 # Güvenlik
│   ├── hash.rs             # BLAKE3
│   ├── auth.rs             # ed25519
│   └── encryption.rs       # E2EE
├── plugins/                # Eklenti sistemi
│   ├── wasm_engine.rs      # wasmtime sandbox (VerbPlugin + LangPlugin + hook'lar)
│   ├── verb_registry.rs    # ~/.something/verbs/ tarayıcı, isim → verb eşleme
│   ├── lang_registry.rs    # ~/.something/langs/ tarayıcı, uzantı → lang eşleme
│   └── hooks_api.rs        # Somewhere ve 3. parti entegrasyon API'si
└── i18n/                   # Çoklu dil desteği
    ├── speak_plugin.rs     # SpeakPlugin trait tanımı
    ├── speak_registry.rs   # ~/.something/translation/ tarayıcı
    └── en.rs               # Built-in İngilizce (fallback)
```

---

## Teknik Stack Özeti

| Katman | Teknoloji | Neden |
|---|---|---|
| Dil | Rust | Memory safety, cross-compile, ekosistem |
| KV Store | Sled → RocksDB | Rust-native, production-proven |
| Hashing | BLAKE3 | 10x SHA-1'den hızlı, paralel |
| Chunking | FastCDC | Content-defined, optimal chunk boyutu |
| AST Motoru | tree-sitter | 40+ dil grammar'ı mevcut |
| Verb Eklentileri | `*.thing` (WASM) | Modüler komutlar, korumalı core |
| Dil Eklentileri | `*.thing` (WASM) | Modüler AST, sandbox, platform bağımsız |
| Sıkıştırma | zstd | En iyi oran/hız dengesi |
| Config | TOML | İnsan okunabilir, yorum destekli |
| i18n | `SpeakPlugin` + `translation/` | Topluluk dil paketleri, RTL dahil |
| Plugin Sandbox | wasmtime | Güvenli izolasyon |
| CLI | clap | Rust standart |

---

## Open-Core Lisans Stratejisi

```
MIT Lisansı (Açık Kaynak)
──────────────────────────
- hey.thing binary (CLI + core motor)
- KV store, BLAKE3, VerbPlugin + LangPlugin trait
- Built-in verb'ler: save, shift, sync, undo, rewind...
- rs.thing — referans dil eklentisi
- Git migration tool
- Tüm CLI komutları (hey save, hey sync...)

Topluluk (thing-langs registry)
────────────────────────────────
- py.thing, js.thing, go.thing... (topluluk eklentileri)
- Herkes kendi *.thing eklentisini yazıp paylaşabilir

Ticari / Kapalı (Somewhere ekosistemi)
────────────────────────────────────────
- Somewhere Hub (SaaS bulut altyapısı)
- Somewhere Projects, Time, Radar
- The Factory (CI/CD)
- Enterprise SSO/SAML
- Resmi dil eklentileri bakımı (py.thing, js.thing vb.)
```

Topluluk hey.thing'e güvenir. thing-langs'e katkı yapar. Somewhere'e ödeme yapar.

---

## Mevcut Durum

Antigravity (Google'ın VS Code forku, Gemini + Claude destekli) ile geliştirme süreci başladı. Mevcut binary debug build halinde çalışıyor. Bir sonraki adım: `Hey` struct ve verb dispatch mimarisinin (`hey save`, `hey sync`...) yazılması; ardından `~/.something/langs/` eklenti sisteminin iskeletinin kurulması.
