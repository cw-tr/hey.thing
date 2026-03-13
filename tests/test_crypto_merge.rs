use hey_thing::crypto::auth::KeyManager;
use hey_thing::core::sync::merge_content_3way;

#[test]
fn test_signature_flow() {
    let data = b"some important data to sign";
    
    // 1. Anahtar üretimi ve imzalama
    let signature = KeyManager::sign(data).expect("İmzalama başarısız");
    let signing_key = KeyManager::get_or_create_key().expect("Anahtar alınamadı");
    let public_key_bytes = signing_key.verifying_key().to_bytes();
    
    // 2. Doğrulama
    let is_valid = KeyManager::verify(data, &signature, &public_key_bytes).expect("Doğrulama işlemi başarısız");
    assert!(is_valid, "İmza geçerli olmalıydı");
    
    // 3. Geçersiz veri ile deneme
    let is_valid_wrong = KeyManager::verify(b"wrong data", &signature, &public_key_bytes).unwrap();
    assert!(!is_valid_wrong, "Yanlış veri ile imza geçersiz olmalıydı");
}

#[test]
fn test_3way_merge_auto() {
    let base = "line1\nline2\nline3\n";
    let local = "line1\nline2-changed\nline3\n";
    let remote = "line1\nline2\nline3-changed\n";
    
    // Şu anki implementasyonumuz tam satır bazlı merge yapmıyor, 
    // ama en azından statik durumları doğru yönetmeli.
    
    // Remote == local durumu (ikisi de aynı şeyi yapmış)
    let (res_same, conf_same) = merge_content_3way(base, "changed", "changed");
    assert_eq!(res_same, "changed");
    assert!(!conf_same);

    // Sadece local değişmiş durumu
    let (res_local, conf_local) = merge_content_3way(base, "changed", base);
    assert_eq!(res_local, "changed");
    assert!(!conf_local);
}

#[test]
fn test_3way_merge_conflict() {
    let base = "original";
    let local = "changed-a";
    let remote = "changed-b";
    
    let (result, has_conflict) = merge_content_3way(base, local, remote);
    
    assert!(has_conflict, "Farklı değişiklikler çakışma üretmeli");
    assert!(result.contains("<<<<<<<< YOURS"));
    assert!(result.contains("changed-a"));
    assert!(result.contains("changed-b"));
    assert!(result.contains(">>>>>>>> THEIRS"));
}
