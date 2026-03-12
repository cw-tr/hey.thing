// tests/test_shift.rs — hey shift komutu entegrasyon testleri

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

fn init_and_save(dir: &std::path::Path) {
    hey(dir, &["init"]);
    fs::write(dir.join("a.txt"), "v1").unwrap();
    hey(dir, &["save", "ilk commit"]);
}

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_shift_between_branches() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "v1 main"]);

    hey(dir.path(), &["shift", "new", "dev"]);
    fs::write(dir.path().join("f.txt"), "v2").unwrap();
    hey(dir.path(), &["save", "v2 dev"]);

    // main'e dön → v1 gelmeli
    let out = hey(dir.path(), &["shift", "main"]);
    assert!(out.status.success());
    let content = fs::read_to_string(dir.path().join("f.txt")).unwrap();
    assert_eq!(content, "v1", "main dalında dosya v1 olmalı");

    // dev'e dön → v2 gelmeli
    hey(dir.path(), &["shift", "dev"]);
    let content = fs::read_to_string(dir.path().join("f.txt")).unwrap();
    assert_eq!(content, "v2", "dev dalında dosya v2 olmalı");
}

#[test]
fn test_shift_updates_head() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "commit 1"]);

    hey(dir.path(), &["shift", "new", "feat"]);
    hey(dir.path(), &["shift", "main"]);

    let head = fs::read_to_string(dir.path().join(".something/HEAD")).unwrap();
    assert!(head.contains("refs/heads/main"), "HEAD main'e işaret etmeli");
}

#[test]
fn test_shift_list_shows_main() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    let out = hey(dir.path(), &["shift"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("main"), "main dalı listelenmeli");
    assert!(stdout.contains("*"), "Aktif dal * ile işaretlenmeli");
}

#[test]
fn test_shift_new_creates_branch() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    let out = hey(dir.path(), &["shift", "new", "feature"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("yolu oluşturuldu"), "Yol oluşturuldu mesajı gelmeli");

    assert!(dir.path().join(".something/refs/heads/feature").exists(),
        "feature dalı refs/heads/ altında oluşturulmalı");
}

#[test]
fn test_shift_new_switches_to_new_branch() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    hey(dir.path(), &["shift", "new", "dev"]);

    let out = hey(dir.path(), &["shift"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("* dev"), "Yeni dal aktif olmalı");
}

#[test]
fn test_shift_duplicate_name_fails() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    hey(dir.path(), &["shift", "new", "test-dal"]);
    let out = hey(dir.path(), &["shift", "new", "test-dal"]);

    assert!(!out.status.success() || {
        let stderr = String::from_utf8_lossy(&out.stderr);
        stderr.contains("zaten mevcut")
    }, "Aynı isimle dal açmak hata vermeli");
}
