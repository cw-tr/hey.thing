# 🛸 hey.thing
**The Intelligent Version Control Engine**  
*Satırların değil, kodun mantığını anlayan yeni nesil VCS çekirdeği.*

---

## ⚡ Nedir?
**hey.thing**, modern yazılım geliştirme süreçleri için tasarlanmış; **Semantic AST Merge**, **Projective VFS** ve **Recursive Delta Compression** teknolojilerini birleştiren hibrit bir versiyon kontrol sistemidir. 

Git'in hantallığından arınmış, büyük veriyi (LFS native) ve akıllı kod birleştirmeyi (AST aware) merkeze alan bir mimari sunar.

---

## ✨ Öne Çıkan Güçler

### 🧬 Semantik Zeka (Phase 3)
Geleneksel "line-based" merge devri kapandı. `hey.thing`, dosyaları birleştirirken kodun yapısını anlar:
*   **WASM Eklenti Sistemi**: Her dil için özel semantik uzmanlar (`.thing` eklentileri).
*   **AST-Aware**: Fonksiyonların yer değiştirmesi veya araya yeni kod girmesi "çakışma" (conflict) yaratmaz.
*   **Diller**: `Rust`, `Python`, `JavaScript/TS`, `PHP` ve `Go` desteği aktif!

### 👻 Projective VFS / Ghost Checkout (Phase 2)
Tera-byte ölçeğindeki repolarda bile saniyeler içinde "checkout" yapın:
*   **Ghost Files**: Proje ağacını 0-byte ghost dosyalarla anında ayağa kaldırır.
*   **On-Demand Materialization**: Sadece açtığınız veya dokunduğunuz dosyalar Hub'dan (`somewhere`) anında indirilir.
*   **FUSE-less**: Kernel modülü gerektirmeyen, tamamen kullanıcı seviyesinde çalışan akıllı VFS.

### 📉 Recursive Delta Compression (Phase 1)
Depolama alanını %90'a varan oranlarda verimli kullanır:
*   **qbsdiff**: Blok bazlı değil, içerik bazlı delta mapping.
*   **Otomatik Repacking**: Delta zincirleri çok uzadığında (max 10) sistemi kasmamak için otomatik snapshot alır.
*   **BLAKE3 Hashing**: SHA-1'den 10 kat daha hızlı ve kriptografik olarak çok daha güvenli.

---

## 🛠️ Komut Seti

| Komut | Açıklama |
| :--- | :--- |
| `hey init` | Yeni bir evren (repo) başlatır. |
| `hey save [msg]` | Mevcut durumu dondurur ve delta-zincirine ekler. |
| `hey shift [hedef]` | Bir dal (branch) veya commit'e geçiş yapar. |
| `hey shift --lazy` | Ghost Checkout modunda saniyeler içinde geçiş yapar. |
| `hey hydrate [yol]` | Ghost dosyayı fiziksel içeriğiyle doldurur. |
| `hey merge [dal]` | İki yolu **Semantik** olarak birleştirir. |
| `hey undo` | Yapılan son işlemi (save, shift vb.) zaman makinesi ile geri alır. |
| `hey lang list` | Yüklü olan Semantik (AST) eklentileri gösterir. |

---

## 🚀 Mevcut Durum: Phase 3 [COMPLETE]
- [x] **Delta & Storage**: QBSDiff entegrasyonu ve sabit RAM mimarisi.
- [x] **VFS Layer**: Ghost worktree ve on-demand fetching.
- [x] **Semantic Merge**: Rust, Py, JS, PHP, Go WASM motorları.
- [x] **Conflict GUI**: Ratatui tabanlı görsel çakışma çözücü.
- [ ] **Phase 4: Remote Hub**: Somewhere Hub v1 API & Real-time Sync (Yakında).

---
*Bu proje **Antigravity** (Advanced Agentic Coding) sistemleri kullanılarak geliştirilmiştir.*
