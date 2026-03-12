// tests/test_setup.rs — hey setup komutu entegrasyon testleri

use std::process::Command;
use tempfile::TempDir;
use std::io::Write;

fn hey_with_stdin(dir: &std::path::Path, args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_hey"))
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("hey çalıştırılamadı");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).unwrap();
    }

    child.wait_with_output().unwrap()
}

// ─── TESTLER ──────────────────────────────────────────────────────────────────

#[test]
fn test_setup_trust_approves() {
    let dir = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_hey"))
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let out = hey_with_stdin(dir.path(), &["setup", "trust"], "e\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("güvenilir olarak işaretlendi"), "Onay mesajı çıkmalı");
}

#[test]
fn test_setup_trust_declines() {
    let dir = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_hey"))
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let out = hey_with_stdin(dir.path(), &["setup", "trust"], "h\n");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("İşlem iptal edildi"), "İptal mesajı çıkmalı");
}
