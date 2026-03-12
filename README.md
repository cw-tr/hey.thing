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

## 🛠️ Mevcut Durum (Faz 1: Çekirdek Motor)

Şu an projenin Çekirdek Motor fazı tamamlanmıştır:
- [x] **Repo Başlatma:** `hey init`
- [x] **Kayıt Sistemi:** `hey save "mesaj"` (Blob ve Tree nesneleri KV store üzerine yazılır)
- [x] **Durum İzleme:** `hey show`
- [x] **Modüler Mimari:** VerbPlugin sistemi kuruldu.

## 📦 Kurulum & Kullanım

Şu an geliştirme aşamasında olduğu için `cargo` üzerinden çalıştırılabilir:

```bash
# Repo başlatma
cargo run -- init

# Değişiklikleri kaydetme
cargo run -- save "İlk commit veritabanına kaydedildi"

# Durumu göster
cargo run -- show
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
