// tests/test_import.rs — hey import komutu entegrasyon testleri

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
fn test_import_no_args_shows_usage() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);

    let out = hey(dir.path(), &["import"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.to_lowercase().contains("import"), "Kullanım mesajı gelmeli");
}

#[test]
fn test_import_from_git_prototype() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);

    let out = hey(dir.path(), &["import", "--from-git", "/tmp/fake"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Migration") || stdout.contains("migration"),
        "Git import prototype çıktısı gelmeli");
}
