// tests/test_save.rs — hey save komutu entegrasyon testleri

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

fn init_repo(dir: &std::path::Path) {
    hey(dir, &["init"]);
}

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_save_with_message() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("file.txt"), "merhaba").unwrap();
    let out = hey(dir.path(), &["save", "İlk kayıt"]);

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Kaydedildi"), "'Kaydedildi' çıktısı olmalı");
}

#[test]
fn test_save_without_message() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("file.txt"), "test").unwrap();
    let out = hey(dir.path(), &["save"]);

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Kaydedildi"), "Mesajsız kayıt da çalışmalı");
}

#[test]
fn test_save_creates_refs_heads_main() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "v1"]);

    assert!(dir.path().join(".something/refs/heads/main").exists(),
        "İlk save sonrası refs/heads/main oluşturulmalı");
}

#[test]
fn test_save_updates_head() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "v1"]);

    let head = fs::read_to_string(dir.path().join(".something/HEAD")).unwrap();
    assert!(head.starts_with("ref: refs/heads/main"),
        "HEAD refs/heads/main'e işaret etmeli");
}
