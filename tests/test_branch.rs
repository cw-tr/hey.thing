// tests/test_branch.rs — hey branch komutu entegrasyon testleri

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
fn test_branch_list_shows_main() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    let out = hey(dir.path(), &["branch"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("main"), "main dalı listelenmeli");
    assert!(stdout.contains("*"), "Aktif dal * ile işaretlenmeli");
}

#[test]
fn test_branch_new_creates_branch() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    let out = hey(dir.path(), &["branch", "new", "feature"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("oluşturuldu"), "Dal oluşturuldu mesajı gelmeli");

    assert!(dir.path().join(".something/refs/heads/feature").exists(),
        "feature dalı refs/heads/ altında oluşturulmalı");
}

#[test]
fn test_branch_new_switches_to_new_branch() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    hey(dir.path(), &["branch", "new", "dev"]);

    let out = hey(dir.path(), &["branch"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("* dev"), "Yeni dal aktif olmalı");
}

#[test]
fn test_branch_duplicate_name_fails() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    hey(dir.path(), &["branch", "new", "test-dal"]);
    let out = hey(dir.path(), &["branch", "new", "test-dal"]);

    assert!(!out.status.success() || {
        let stderr = String::from_utf8_lossy(&out.stderr);
        stderr.contains("zaten mevcut")
    }, "Aynı isimle dal açmak hata vermeli");
}
