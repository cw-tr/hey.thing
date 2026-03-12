// tests/test_rewind.rs — hey rewind komutu entegrasyon testleri

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

fn get_current_hash(dir: &std::path::Path) -> String {
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
fn test_rewind_to_specific_commit() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "commit 1"]);
    let hash1 = get_current_hash(dir.path());

    fs::write(dir.path().join("f.txt"), "v2").unwrap();
    hey(dir.path(), &["save", "commit 2"]);

    // hash1'e rewind
    let out = hey(dir.path(), &["rewind", &hash1]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("geri sarıldı"), "'geri sarıldı' mesajı gelmeli");

    let current = get_current_hash(dir.path());
    assert_eq!(current, hash1, "Rewind sonrası hash eşleşmeli");
}

#[test]
fn test_rewind_invalid_hash_fails() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "commit 1"]);

    let out = hey(dir.path(), &["rewind", "gecersiz_hash_1234"]);
    assert!(!out.status.success() || {
        let stderr = String::from_utf8_lossy(&out.stderr);
        stderr.contains("bulunamadı")
    }, "Geçersiz hash ile rewind hata vermeli");
}
