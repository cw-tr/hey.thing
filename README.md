# hey.thing

**Açık Kaynak Versiyon Kontrol Motoru**  
*Mukan Erkin tarafından tasarlanan modern VCS çekirdeği.*

---

## 🚀 Nedir bu?

**hey.thing**, Git'in 20 yıllık tasarım borçlarını sıfırdan çözen; modern kriptografi, semantik kod birleştirme ve akıllı CLI mimarisiyle tasarlanmış, tamamen açık kaynaklı bir Versiyon Kontrol Sistemi motorudur.

`hey.thing`, Somewhere ekosisteminin açık kaynak çekirdeğidir — ama bağımsız olarak da tam işlevsel bir VCS'dir.

## ✨ Öne Çıkan Özellikler

- **Rust ile Güvenli:** Bellek güvenliği ve performans için Rust (Edition 2024) ile geliştirildi.
- **Hızlı Depolama:** Binlerce küçük dosya yerine tek bir optimize edilmiş KV (Sled/RocksDB) deposu.
- **Güçlü Hashing:** SHA-1 yerine 10 kat daha hızlı ve güvenli olan **BLAKE3**.
- **Modern CLI:** Tek binary, modüler "verb" eklentileri (`*.thing`).
- **Semantik AST Merge:** Sadece satırları değil, dili anlayan akıllı birleştirme sistemi.
- **Event Journal:** Zaman makinesi desteği için her eylemin kaydı (`hey undo`).

## 🛠️ Mevcut Durum (Faz 1, 2, 3 ve 4.1 Tamamlandı)

Şu an projenin Çekirdek, Geçmiş İzleme, Ağ Senkronizasyonu ve Semantik Zeka (WASM Plugin Engine) fazları geliştirilmektedir:
- [x] **Repo Başlatma & Güvenlik:** `hey init`, `hey setup trust`
- [x] **Kayıt Sistemi:** `hey save "mesaj"` (Blob ve Tree nesneleri KV store üzerinde zstd ile sıkıştırılır, chunklanır)
- [x] **Dallanma (Branching) & Shift:** `hey shift` ile yollar (dallar) arasında geçiş yapma ve oluşturma
- [x] **Zaman Makinesi:** `hey rewind` ve `hey undo`
- [x] **Git Göçü (Migration):** `hey import --from-git` (Mevcut git objelerini içeri aktarır)
- [x] **Delta Senkronizasyon:** `hey sync` ve `hey get` komutlarıyla Uzak HTTP Hub'lara doğrudan push/pull yapabilme.
- [x] **Çakışma Toleransı ve Otomatik Merge:** `hey get` komutuyla veri çekilirken akıllı 3-Way merge yapılması (Diverge vs Fast-Forward algılama).
- [x] **Semantik AST Motoru (Faz 4.1):** `wasmtime` tabanlı `LangPlugin` altyapısı sayesinde eklenti tabanlı AST formatında merge yeteneği.

## 📦 Kurulum & Kullanım

Şu an geliştirme aşamasında olduğu için kaynak koddan derleyerek kullanabilirsiniz:

```bash
# Bilgisayarınıza hey olarak derleyip kuralım
cargo install --path .

# Repo başlatma
hey init

# Dosya oluşturup kaydetme
hey save "İlk commit veritabanına kaydedildi"

# Yeni bir yol (dal) oluşturup ona geçiş yapma
hey shift new test-dali

# Hub yapılandırmasını ayarla (.configthing içinde [somewhere] bloğu)
# hey sync artık parametresiz olarak önce backup'a sonra remote'a aktarır
hey sync

# Belirli bir hedefe senkronize et
hey sync backup
hey sync remote

# Uzak sunucudaki güncellemeleri çek (pull/get)
hey get remote
```

## 🏗️ Mimari Şema

```
~/.something/
├── bin/
│   └── hey.thing            ← PATH'e eklenen tek binary
├── core/
│   └── the-thing            ← VCS motoru
├── verbs/                   ← verb eklentileri (save.thing, shift.thing...)
├── langs/                   ← AST dil eklentileri (rs.thing, py.thing...)
└── config/
    └── global.toml          ← Kullanıcı geneli ayarlar
```

## 📄 Lisans

Bu proje **MIT** lisansı ile lisanslanmıştır. Detaylar için [LICENSE](LICENSE) dosyasına bakınız.

- [hey.thing Teknik Tanıtım](docs/technical_spec.md)
- [Geliştirme Yol Haritası](docs/roadmap.md)

---
*Geliştirme süreci Antigravity (Advanced Agentic Coding) ile devam etmektedir.*
