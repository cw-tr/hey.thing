// tests/test_show.rs — hey show komutu entegrasyon testleri

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
    fs::write(dir.join("a.txt"), "test").unwrap();
    hey(dir, &["save", "test commit"]);
}

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_show_displays_commit_info() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    let out = hey(dir.path(), &["show"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(stdout.contains("Son Commit:"), "'Son Commit:' satırı olmalı");
    assert!(stdout.contains("Yazar:"), "'Yazar:' satırı olmalı");
    assert!(stdout.contains("Mesaj:"), "'Mesaj:' satırı olmalı");
    assert!(stdout.contains("Dal:"), "'Dal:' satırı olmalı");
}

#[test]
fn test_show_displays_correct_message() {
    let dir = TempDir::new().unwrap();
    init_and_save(dir.path());

    let out = hey(dir.path(), &["show"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("test commit"), "Mesaj doğru görünmeli");
}
