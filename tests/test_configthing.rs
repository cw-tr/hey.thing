// tests/test_configthing.rs — .configthing dosyası entegrasyon testleri

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
fn test_configthing_exists_after_init() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);
    assert!(dir.path().join(".configthing").is_file(),
        ".configthing dosyası init sonrası oluşturulmalı");
}

#[test]
fn test_configthing_has_user_section() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);
    let content = fs::read_to_string(dir.path().join(".configthing")).unwrap();
    assert!(content.contains("[user]"), "[user] bölümü olmalı");
    assert!(content.contains("name"), "name alanı olmalı");
}

#[test]
fn test_configthing_has_behavior_section() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);
    let content = fs::read_to_string(dir.path().join(".configthing")).unwrap();
    assert!(content.contains("[behavior]"), "[behavior] bölümü olmalı");
    assert!(content.contains("auto_stage_all"), "auto_stage_all alanı olmalı");
    assert!(content.contains("ignore_empty_commits"), "ignore_empty_commits alanı olmalı");
    assert!(content.contains("journal_retention"), "journal_retention alanı olmalı");
}
