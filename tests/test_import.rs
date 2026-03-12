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
fn test_import_from_git_invalid_path() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);

    let out = hey(dir.path(), &["import", "--from-git", "/tmp/nonexistent"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Git reposu değil") || !out.status.success(),
        "Geçersiz yol hata vermeli");
}

#[test]
fn test_import_from_real_git_repo() {
    let dir = TempDir::new().unwrap();
    hey(dir.path(), &["init"]);

    // Gerçek bir mini git reposu oluştur
    let git_dir = TempDir::new().unwrap();
    Command::new("git").args(["init"]).current_dir(git_dir.path()).output().unwrap();
    Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(git_dir.path()).output().unwrap();
    Command::new("git").args(["config", "user.name", "Test"]).current_dir(git_dir.path()).output().unwrap();
    std::fs::write(git_dir.path().join("hello.txt"), "merhaba").unwrap();
    Command::new("git").args(["add", "."]).current_dir(git_dir.path()).output().unwrap();
    Command::new("git").args(["commit", "-m", "ilk commit"]).current_dir(git_dir.path()).output().unwrap();

    let git_path = git_dir.path().to_string_lossy().to_string();
    let out = hey(dir.path(), &["import", "--from-git", &git_path]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tamamlandı"), "Import tamamlandı mesajı gelmeli");
    assert!(stdout.contains("1 commit"), "1 commit aktarılmalı");
}
