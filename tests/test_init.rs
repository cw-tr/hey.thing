// tests/test_init.rs — hey init komutu entegrasyon testleri

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

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_init_creates_something_dir() {
    let dir = TempDir::new().unwrap();
    let out = hey(dir.path(), &["init"]);
    assert!(out.status.success());
    assert!(dir.path().join(".something").is_dir(),
        ".something dizini oluşturulmalı");
}

#[test]
fn test_init_creates_configthing() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);
    assert!(dir.path().join(".configthing").is_file(),
        ".configthing dosyası oluşturulmalı");
}

#[test]
fn test_init_configthing_has_correct_sections() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);
    let content = fs::read_to_string(dir.path().join(".configthing")).unwrap();
    assert!(content.contains("[user]"), "[user] bölümü olmalı");
    assert!(content.contains("[behavior]"), "[behavior] bölümü olmalı");
    assert!(content.contains("auto_stage_all"), "auto_stage_all alanı olmalı");
    assert!(content.contains("journal_retention"), "journal_retention alanı olmalı");
}

#[test]
fn test_init_idempotent() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);
    let out = hey(dir.path(), &["init"]);
    assert!(out.status.success(), "İkinci init hata vermemeli");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("zaten mevcut"), "Zaten mevcut mesajı gelmeli");
}
