// tests/test_undo.rs — hey undo komutu entegrasyon testleri

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
fn test_undo_reverts_to_parent() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "v1").unwrap();
    hey(dir.path(), &["save", "commit 1"]);

    fs::write(dir.path().join("f.txt"), "v2").unwrap();
    hey(dir.path(), &["save", "commit 2"]);

    let out = hey(dir.path(), &["undo"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("geri alındı"), "'geri alındı' mesajı gelmeli");

    // Show ile kontrol — commit 1 mesajı görünmeli
    let show = hey(dir.path(), &["show"]);
    let show_out = String::from_utf8_lossy(&show.stdout);
    assert!(show_out.contains("commit 1"), "Undo sonrası ilk commit mesajı görünmeli");
}

#[test]
fn test_undo_first_commit_fails() {
    let dir = TempDir::new().unwrap();

    hey(dir.path(), &["init"]);
    fs::write(dir.path().join("f.txt"), "only").unwrap();
    hey(dir.path(), &["save", "tek commit"]);

    let out = hey(dir.path(), &["undo"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ilk commit") || !out.status.success(),
        "İlk commit'te undo hata vermeli");
}
