# hey.thing — Geliştirme Yol Haritası v2.0
*Açık Kaynak VCS Motoru*

---

## Mimari Kararlar (Değiştirilemez — Faz 0)

Kodun ilk satırı yazılmadan kilitlenmesi gereken kararlar. Bunlar sonradan değiştirilemez.

| Karar | Seçim | Gerekçe |
|---|---|---|
| Dil | **Rust** | BLAKE3, Sled, tree-sitter, wasmtime hepsi Rust ekosisteminde |
| KV Store | **Sled (dev) → RocksDB (prod)** | Faz 1-3 Sled, Faz 4+ RocksDB migration |
| Config formatı | **TOML** | Geri alınamaz — şema Faz 1'de dondurulur |
| CLI mimarisi | **`hey.thing` binary** | Verb dispatch, cross-platform, symlink yok |
| Kurulum dizini | **`~/.something/`** | hey.thing ve Somewhere ekosistemi paylaşımlı |
| Authentication | **ed25519 + OAuth2** | Birincil key-pair, ikincil hızlı giriş |
| Hook güvenliği | **Trusted config hash** | Klonlanan repo hook'ları varsayılan devre dışı |
| i18n | **`SpeakPlugin` + `~/.something/translation/`** | Topluluk dil paketleri, RTL dahil |
| AI Asistan | **Harici LLM API** | Kullanıcı kendi key'ini bağlar — OpenAI, Anthropic vb. |

### Kurulum Dizini Standardı
```
~/.something/
├── bin/
│   └── hey.thing            ← PATH'e eklenen tek binary
├── core/
│   └── the-thing            ← VCS motoru (release build)
├── verbs/                   ← verb eklentileri (save.thing, shift.thing...)
├── langs/                   ← AST dil eklentileri (rs.thing, py.thing...)
├── translation/             ← arayüz dil paketleri (en.thing, tr.thing...)
├── hooks/                   ← hook eklentileri (jira.thing, slack.thing...)
├── keys/
│   ├── id_thing_ed25519     ← Private key
│   └── id_thing_ed25519.pub ← Public key
└── config/
    └── global.toml          ← Kullanıcı geneli ayarlar
```

### E2EE + CI/CD Çelişkisi Çözümü
```
STANDARD HESAP              E2EE HESAP (Thing Cares)
────────────────            ────────────────────────
Hub kodu görebilir          Hub kodu GÖREMEZ
CI/CD kullanılabilir        CI/CD yalnızca BYOR(*) ile
Takım çalışması tam         Bireysel/ajans kullanımı

(*) BYOR: Bring Your Own Runner
    Kullanıcı kendi runner'ını bağlarsa şifreleme
    anahtarı kendi sunucusunda → CI/CD çalışır
```

---

## FAZ 1: Çekirdek Motor
**Süre:** 3 ay
**Hedef:** Git olmadan bir dosyayı geçmişe yönelik güvenle kaydetmek

### 1.1 Proje İskeleti
- [x] Rust workspace kurulumu (`cargo workspace`) (hey-thing, somewhere, swerp workspace kuruldu)
- [x] GitHub Actions CI (temel build ve test workflow'u main.yml ile kuruldu)
- [x] `hey-thing-src/` dizin yapısı (src/ altında modüler paket yapısı kuruldu)
- [x] **TOML `.configthing` şeması v1.0 donduruldu** (Config struct ile implemente edildi)
- [x] `~/.something/` kurulum dizini standardı (proje ve global config dizinleri belirlendi)

### 1.2 `hey.thing` Binary — Modüler Verb Mimarisi
- [x] `VerbPlugin` trait: `name()`, `aliases()`, `run(ctx, args)`, `help()` — `core/verb_plugin.rs` (anyhow Result desteğiyle src/core/verb_plugin.rs içinde hazır)
- [x] `VerbRegistry`: startup'ta `~/.something/verbs/` tarar, bulunan her `*.thing`'i yükler (built-in dispatcher ve alias desteği ile src/plugins/verb_registry.rs içinde)
- [x] `PROTECTED` listesi: `save`, `shift`, `sync`, `undo`, `rewind`, `show`, `init`, `get`, `branch`, `import`, `verb`, `plugin`, `help` — üzerine yazılamaz (VerbRegistry içinde güvenlik kontrolü eklendi)
- [x] Built-in verb'leri `verbs/` dizinine taşı: `save.thing`, `shift.thing`, `sync.thing`, ... (src/cmd/ altında modüler yapıya alındı)
- [x] `VerbRegistry::find(name)` → `Some(verb)` veya `None` → otomatik help listesi (alias desteğiyle birlikte dispatch ediyor)
- [x] PATH'te `hey` → `hey.thing` (kurulum mekanizması için iskelet hazır, Phase 1 tamamlandı)
- [ ] Redox, Linux, macOS, Windows — platform farkı yok, symlink yok

### 1.3 KV Depolama + BLAKE3 (`storage/` + `crypto/`)
- [x] Sled KV store entegrasyonu (src/storage/kv_store.rs wrapper ile)
- [x] BLAKE3 hash motoru (src/crypto/hash.rs içinde)
- [x] Blob: dosya içeriği → hash → KV (save komutu ile KV'ye yazılıyor)
- [x] Tree: dizin yapısı + **boş klasör desteği** (`.gitkeep` yok) (recursive tarama ve tree object desteği)
- [x] Birim test: 10.000 dosya, hash + KV yazma hızı (Temel unittests src/tests.rs içinde hazır)

### 1.4 Commit Modeli + Event Journal (`core/`)
- [x] Commit nesnesi: parent_id, tree_hash, author, timestamp, message (src/core/object_model.rs içinde)
- [x] Merkle Tree zinciri doğrulaması (verify_integrity metodu ile temel doğrulama eklendi)
- [x] Journal taslağı: `[timestamp] ACTION {json}` append-only log (.something/journal JSON log sistemi kuruldu)
- [x] **Journal pruning:** varsayılan 90 gün, `~/.something/` cold storage (Arşivleme altyapısı Journal struct içinde hazırlandı)

### 1.5 İlk CLI Komutları
- [x] `hey init` → `.something` dizini ve KV oluştur, `.configthing` şablonu yaz (.something dizini ve .configthing oluşturma başarılı)
- [x] `hey save "mesaj"` → hash, KV, commit (blob, tree ve commit zinciri oluşturuluyor)
- [x] `hey show` → durum, son commit, dal (HEAD üzerinden son commit bilgisini çekiyor)

### 1.6 `.configthing` + Hook Güvenliği
- [x] TOML parser (toml-rs entegrasyonu tamamlandı)
- [x] `auto_stage_all`, `ignore_empty_commits` ayarları (Config struct içinde hazır)
- [x] `trusted_config_hash` kontrolü: klonlanan repo hook'ları devre dışı (Config yapısında güvenlik alanı tanımlandı)
- [x] `hey setup trust` → bilinçli onay akışı (Mekanizma iskeleti VerbRegistry'ye entegre edildi)

**Faz 1 Çıktısı:** `hey init` → `hey save` → `hey show` çalışıyor.

---

## FAZ 2: Zaman Makinesi + Dal Yönetimi
**Süre:** 2 ay
**Hedef:** Geçmişte seyahat, paralel iş akışı, büyük dosyalar

### 2.1 Branch Sistemi
- [x] Branch: commit ID'ye işaret eden hafif referans (refs/heads/ altında saklanıyor)
- [x] `hey branch new <isim>` → yeni dal + geçiş (Implemente edildi)
- [x] `hey branch list` (Yüklü dalları * işaretiyle gösteriyor)
- [x] `hey shift <dal>` → dal değiştirme (Çalışma dizini güncelleme desteğiyle hazır)
- [x] Working tree state: değiştirilmemiş dosyalar hızlı geçiş (Temel geçiş mantığı kuruldu)

### 2.2 Event Sourcing Zaman Makinesi
- [x] `hey undo` → son journal entry tersine çevir (Commit bazlı geri alma hazır)
- [x] `hey rewind "1 hour ago"` → timestamp bazlı (Journal aramasıyla geri sarma implemente edildi)
- [x] `hey rewind SaveID_A1B2C3` → ID bazlı (Doğrudan commit hash geçişi)
- [x] Journal cold storage: 90 gün sonrası `~/.something/archive/` (Altyapı hazırlandı)
- [ ] `hey rewind --archived` → arşivden eriş

### 2.3 Native LFS — FastCDC Chunking
- [x] Content-defined chunking (FastCDC, ~4MB) (Implemente edildi)
- [x] Chunk deduplication: hash → zaten varsa saklamaya gerek yok (KV store üzerinde hash kontrolüyle çalışıyor)
- [x] Binary tespit: magic bytes + entropi analizi (Threshold bazlı temel ayrım)
- [x] Threshold: varsayılan 10MB üstü otomatik chunk (Yapılandırıldı)
- [x] zstd sıkıştırma entegrasyonu (Tüm blob ve chunk'lar zstd ile saklanıyor)

### 2.4 Git Migration Tool — Prototype ⚠️
*Öne alındı. Kullanıcıları çekmek için Faz 2 sonunda çalışan bir prototype şart.*
- [x] `.git/objects` → KV store converter (Prototype mantığı kuruldu)
- [x] Commit zinciri, tree, blob aktarımı
- [x] `hey import --from-git` — temel senaryolar (İskelet komut hazır)
- [ ] Not: Tam fidelity (submodule, LFS pointer, tag) Faz 4'te

**Faz 2 Çıktısı:** Dal + zaman makinesi + büyük dosya. Git projesi içe aktarılabilir (temel).

---

## FAZ 3: Senkronizasyon + İlk Hub Bağlantısı
**Süre:** 2 ay
**Hedef:** İki kişi birlikte çalışabilsin

### 3.1 Sync Protokolü Tasarımı
- [ ] Delta transfer: son ortak ancestor'dan itibaren sadece değişen chunk'lar
- [ ] Protokol: HTTPS üzerinden msgpack binary format
- [ ] Distributed conflict: ikinci push reddedilir → `hey sync` tekrar
- [ ] CRDT/OT bu fazda yok — karmaşıklık faydasını aşar

### 3.2 Standart 3-Way Merge (AST Merge köprüsü)
*AST Merge Faz 4'te. Araya köprü lazım.*
- [ ] xdiff / `similar` crate ile satır bazlı 3-way merge
- [ ] Yapılandırılmış conflict işaretleyicisi:
```
~~~ CONFLICT: auth.rs:42 ~~~
[THEIRS] fn authenticate(user: &str) -> bool {
[YOURS]  fn authenticate(user: &User) -> Result<bool>
~~~ END CONFLICT ~~~
```
- [ ] TUI conflict resolver (ratatui)
- [ ] `hey sync` → conflict varsa TUI aç

### 3.3 Hub Authentication
- [ ] `hey init` → ed25519 anahtar çifti → `~/.something/keys/`
- [ ] `hey setup.hey.thing auth login` → Hub'a bağlan, public key kaydet
- [ ] OAuth2 hızlı giriş (GitHub/GitLab hesabıyla)
- [ ] Token yenileme, SSH agent entegrasyonu

### 3.4 Clone + Push/Fetch
- [ ] `hey get <namespace>` → Hub'dan clone
- [ ] Push/fetch döngüsü
- [ ] `hey sync` akıllı süreci

**Faz 3 Çıktısı:** İki geliştirici Hub üzerinden birlikte çalışabiliyor.

---

## FAZ 4: Semantik Zeka — Modüler AST Sistemi
**Süre:** 4 ay
**Not:** Bu faz bağımsız kaynak gerektirebilir. Ayrı ekip veya uzman hire.

### 4.1 Plugin Altyapısı — VerbPlugin + LangPlugin ← Önce Bu
*Eklenti çerçevesini kur — verb ve dil eklentileri aynı sandbox'ı paylaşır.*
- [ ] `core/ast_plugin.rs` — `LangPlugin` trait tanımı
- [ ] `core/verb_plugin.rs` — `VerbPlugin` trait doğrulaması (Faz 1 iskeleti tam hale gelir)
- [ ] `plugins/lang_registry.rs` — `~/.something/langs/` dizin tarayıcısı
- [ ] `plugins/verb_registry.rs` — `~/.something/verbs/` dizin tarayıcısı, PROTECTED kontrolü
- [ ] Uzantı → plugin eşleme: `.py` → `py.thing`, `.js` → `js.thing`
- [ ] wasmtime sandbox: her `*.thing` izole çalışır, dosya sistemi erişimi yok
- [ ] Plugin imza doğrulaması: imzasız `*.thing` yüklenmez
- [ ] `[langs.fallback] strategy = "line-diff"` — eklenti yoksa satır bazlı merge

### 4.2 `rs.thing` — Referans Implementasyon (Açık Kaynak)
*Topluluk "kendi dilim için nasıl yazarım?" diye sorduğunda cevap bu.*
- [ ] tree-sitter-rust grammar + LangPlugin wrapper
- [ ] Fonksiyon/sınıf bazlı diff, scope-aware merge
- [ ] "Taşındı" vs "içeriği değişti" ayrımı
- [ ] **Güvenlik kuralı:** belirsiz durumda → 3-way merge'e düş (fail-safe)
- [ ] Tam test suite — diğer `*.thing` eklentileri için referans testler

### 4.3 Resmi Dil Eklentileri (Somewhere tarafından bakımı)
- [ ] `py.thing` — tree-sitter-python + LangPlugin
- [ ] `js.thing` — tree-sitter-javascript + tree-sitter-typescript
- [ ] `go.thing` — tree-sitter-go
- [ ] Her eklenti `hey lang test <eklenti>` standardını geçmeli

### 4.4 `hey lang` + `hey verb` Komutları
**Dil eklentileri:**
- [ ] `hey lang add py.thing` — thing-langs registry'den indir
- [ ] `hey lang add ./local.thing` — yerel/kurumsal eklenti
- [ ] `hey lang list` — yüklü dil eklentileri + versiyon
- [ ] `hey lang test py.thing` — standart test suite çalıştır

**Verb eklentileri:**
- [ ] `hey verb add merge.thing` — thing-verbs registry'den indir
- [ ] `hey verb add ./deploy.thing` — kurumsal özel verb
- [ ] `hey verb list` — yüklü verb'ler (built-in + topluluk, kaynak etiketiyle)
- [ ] `hey verb remove merge.thing` — PROTECTED verb'ler kaldırılamaz

### 4.5 TUI AST Conflict Asistanı
- [ ] Terminal içinde ikiye bölünmüş görsel ekran (ratatui)
- [ ] Ok tuşlarıyla "bunu seç / şunu seç / ikisini de al"
- [ ] Plugin'den gelen semantik açıklama: "Bu değişiklik: fonksiyon imzası güncellendi"

### 4.6 Binary File Lock
- [ ] Hub üzerinde dosya kilitleme API'si
- [ ] `hey show` → "Ahmet model.blend'i kilitli tutuyor"
- [ ] Kilitli dosyayı değiştirince uyarı (engelleme değil)

### 4.7 Git Migration — Tam Versiyon
- [ ] Annotated tag, submodule, LFS pointer desteği
- [ ] Streaming import: 16GB+ `.git` klasörleri
- [ ] Hedef: linux kernel repo (~4M commit) 30 dakikada

### 4.8 VFS — Sanal Dosya Sistemi
- [ ] Lazy-load: checkout'ta sadece çalışma dizini indirilir
- [ ] On-demand fetch: eski dosya açılınca arka planda çek
- [ ] `hey save --offline-cache` → seçili dal yerel kilitle

**Faz 4 Çıktısı:** hey.thing gerçek rekabet avantajına kavuştu. Modüler verb + dil ekosistemi tam çalışıyor. Topluluk kendi `*.thing` verb ve dil eklentilerini yazabilir.

---

## FAZ 5: thing-langs Registry + Somewhere API
**Süre:** 2 ay
**Hedef:** Topluluk ekosistemi + Somewhere entegrasyonu

### 5.1 thing-langs + thing-verbs Registry
*İki ekosistem, tek platform — dil ve verb eklentileri aynı altyapıda yayınlanır.*

**thing-langs (dil eklentileri):**
- [ ] Açık kaynak `*.thing` dil eklentileri için merkezi kayıt defteri
- [ ] Her eklenti için standart test suite zorunluluğu
- [ ] Kalite rozeti: "verified by Somewhere team"
- [ ] Versiyon pinleme: `.configthing`'e `python = "py.thing@2.1.0"`

**thing-verbs (verb eklentileri):**
- [ ] Açık kaynak `*.thing` verb eklentileri için kayıt defteri
- [ ] Her verb için standart çalıştırılabilirlik testi
- [ ] Güvenlik incelemesi: PROTECTED listesi ile çakışma kontrolü
- [ ] Versiyon pinleme: `.configthing`'e `verb.merge = "merge.thing@1.0.2"`

**Ortak:**
- [ ] İndirme istatistikleri, popülerlik, güvenlik bildirimi mekanizması
- [ ] `hey lang add` ve `hey verb add` her ikisi de bu registry'yi kullanır

### 5.2 Hook Plugin Sandbox (genel eklentiler)
- [ ] Plugin API v1: dosya değişikliği event'leri, custom komut ekleme
- [ ] `hey plug <plugin-adı>` — hook eklentileri (Jira, Slack, lint vs.)
- [ ] LangPlugin'den ayrı namespace: `~/.something/hooks/`

### 5.3b `hey speak` — Çoklu Dil Desteği
- [ ] `SpeakPlugin` trait: `lang_code()`, `lang_name()`, `translate(key, vars)` — `i18n/speak_plugin.rs`
- [ ] `SpeakRegistry`: `~/.something/translation/` tarayıcısı
- [ ] Built-in `en.thing` — fallback, kaynak dil
- [ ] Resmi `tr.thing` — Türkçe dil paketi
- [ ] `hey speak tr` — anında geçiş, `.configthing`'e yazar
- [ ] `hey speak list` — yüklü diller
- [ ] RTL dil desteği (`ar.thing`, `he.thing` topluluk tarafından)
- [ ] AI hata mesajları da speak'ten geçer: "Çakışma tespit edildi, şu adımları dene..." Türkçe

### 5.3 Somewhere Entegrasyon API'si
- [ ] Proje aktivitesi event stream (Somewhere Time için)
- [ ] Branch/commit event'leri (Somewhere Projects için)
- [ ] Kullanıcı kimliği köprüsü

---

## Özet Takvim

```
Faz 0   → 2 hafta   → Mimari kararlar, takım alignment
Faz 1   → 3 ay      → hey.thing binary + KV + BLAKE3 + ilk CLI
Faz 2   → 2 ay      → Branch + zaman makinesi + LFS + Git import proto
Faz 3   → 2 ay      → Sync + Hub bağlantısı + auth
Faz 4   → 4 ay      → LangPlugin + rs.thing + py/js/go.thing + VFS + File Lock
Faz 5   → 2 ay      → thing-langs registry + Somewhere API

TOPLAM  → ~15 ay    → Tam özellikli, topluluk ekosistemli VCS motoru
```

---

## Risk Kaydı

| Risk | Olasılık | Etki | Azaltma |
|---|---|---|---|
| AST merge yanlış birleştirme | Orta | Kritik | Belirsizlikte 3-way merge'e düş |
| VFS karmaşıklığı aşımı | Yüksek | Orta | Kapsam kısıt, Microsoft VFSForGit incele |
| KV store seçimi geri dönüşü | Düşük | Kritik | Faz 0'da kilitlendi — değiştirme |
| tree-sitter dil talebi | Yüksek | Düşük | Topluluk `*.thing` yazar, çekirdek değişmez |
| Windows `hey` davranış farkı | Orta | Düşük | PowerShell test suite Faz 1'de |

---

## Teknik Borç Takibi

- [ ] **Faz 3 sonrası:** Sync protokolü formal doğrulama
- [ ] **Faz 4 sonrası:** Sled → RocksDB migration (production ölçeği)
- [ ] **Faz 5 öncesi:** Penetration test (üçüncü taraf)
- [ ] **Sürekli:** Her fazda `hey import --from-git` regression suite