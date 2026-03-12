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

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_shift_between_branches() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "v1 main"]);

    hey(dir.path(), &["branch", "new", "dev"]);
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

    hey(dir.path(), &["branch", "new", "feat"]);
    hey(dir.path(), &["shift", "main"]);

    let head = fs::read_to_string(dir.path().join(".something/HEAD")).unwrap();
    assert!(head.contains("refs/heads/main"), "HEAD main'e işaret etmeli");
}
