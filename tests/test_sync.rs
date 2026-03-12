// tests/test_sync.rs — hey sync komutu yerel delta transferi entegrasyon testleri

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn hey(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_hey"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("hey binary çalıştırılamadı")
}

fn get_head(dir: &std::path::Path) -> String {
    let out = hey(dir, &["show"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.starts_with("Son Commit:") {
            return line.split(':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    String::new()
}

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_sync_local_to_local_empty_remote() {
    let repo_a = TempDir::new().unwrap();
    let repo_b = TempDir::new().unwrap();

    // Repo A'yı hazırla
    hey(repo_a.path(), &["init"]);
    fs::write(repo_a.path().join("a.txt"), "hello from A").unwrap();
    hey(repo_a.path(), &["save", "commit 1 in A"]);

    let head_a = get_head(repo_a.path());

    // Repo B'yi hazırla (boş)
    hey(repo_b.path(), &["init"]);

    // A'dan B'ye local push
    let b_path = repo_b.path().to_string_lossy().to_string();
    let out = hey(repo_a.path(), &["sync", &b_path]);

    assert!(out.status.success(), "Sync komutu başarıyla tamamlanmalı");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Aktarılacak paket boyutu"), "Delta paket boyutu hesaplanmalı");
    assert!(stdout.contains("1 commit"), "İlk aktarımda 1 commit gitmeli");

    let head_b = get_head(repo_b.path());
    assert_eq!(head_a, head_b, "Sync sonrası A ve B'nin HEAD hashleri aynı olmalı");
}

#[test]
fn test_sync_local_to_local_fast_forward() {
    let repo_a = TempDir::new().unwrap();
    let repo_b = TempDir::new().unwrap();

    // 1. A ve B aynı kökten başlasın (A'dan B'ye kopyalama işlemi yerine, B bomboş ama senkronize olmuş)
    hey(repo_a.path(), &["init"]);
    hey(repo_b.path(), &["init"]);

    fs::write(repo_a.path().join("a.txt"), "v1").unwrap();
    hey(repo_a.path(), &["save", "v1"]);

    let b_path = repo_b.path().to_string_lossy().to_string();
    hey(repo_a.path(), &["sync", &b_path]);

    // 2. A'ya yeni bir commit daha at, B sadece yeni commit'i almalı
    fs::write(repo_a.path().join("a.txt"), "v2").unwrap();
    hey(repo_a.path(), &["save", "v2"]);
    let head_a2 = get_head(repo_a.path());

    let out = hey(repo_a.path(), &["sync", &b_path]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("1 commit"), "Yalnızca eksik olan 1 son commit aktarılmalı");

    let head_b2 = get_head(repo_b.path());
    assert_eq!(head_a2, head_b2, "B güncellenmeli");
}

#[test]
fn test_sync_conflict_detection() {
    let repo_a = TempDir::new().unwrap();
    let repo_b = TempDir::new().unwrap();
    hey(repo_a.path(), &["init"]);
    hey(repo_b.path(), &["init"]);

    // Ortak kök
    fs::write(repo_a.path().join("base.txt"), "base").unwrap();
    hey(repo_a.path(), &["save", "base"]);
    let b_path = repo_b.path().to_string_lossy().to_string();
    hey(repo_a.path(), &["sync", &b_path]);

    // A kendi yolunda ilerler
    fs::write(repo_a.path().join("a_only.txt"), "A").unwrap();
    hey(repo_a.path(), &["save", "A diverged"]);

    // B kendi yolunda ilerler (B'nin workspace'inde olduğu için B'yi modife edelim)
    // Aslında remote_repo'ya commit atabilmek için b_path'de çalışmamız lazım
    hey(repo_b.path(), &["shift", "main"]); // çalışma klasörünü sync'e senkronladık
    // B'ye yeni commit atalım (hey b dizininde çalışır)
    fs::write(repo_b.path().join("b_only.txt"), "B").unwrap();
    hey(repo_b.path(), &["save", "B diverged"]);

    // A'dan B'ye sync denemesi: ÇATIŞMA vermeli
    let out = hey(repo_a.path(), &["sync", &b_path]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    
    assert!(stdout.contains("ÇATIŞMA") || stdout.contains("Conflict"), "Conflict uyarısı alınmalı");
}
