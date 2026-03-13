use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use std::fs;
use anyhow::{Result, anyhow};
use crate::core::paths;

pub struct KeyManager;

impl KeyManager {
    /// Mevcut anahtarı yükler veya yoksa yeni bir tane üretir.
    pub fn get_or_create_key() -> Result<SigningKey> {
        let key_path = paths::get_default_key_path();
        
        if key_path.exists() {
            let bytes = fs::read(&key_path)?;
            if bytes.len() == 32 {
                let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| anyhow!("Geçersiz anahtar formatı"))?;
                return Ok(SigningKey::from_bytes(&bytes_array));
            }
        }

        // Yeni anahtar üret
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        
        // Klasörleri hazırla
        paths::ensure_global_dirs()?;
        
        // Kaydet (Sadece 32 byte'lık secret key'i kaydediyoruz)
        fs::write(&key_path, signing_key.to_bytes())?;
        
        // Public key'i de kolaylık olsun diye yanına kaydedelim
        let pub_path = key_path.with_extension("pub");
        fs::write(pub_path, signing_key.verifying_key().to_bytes())?;

        Ok(signing_key)
    }

    /// Veriyi imzalar ve Base64 formatında döner
    pub fn sign(data: &[u8]) -> Result<String> {
        let key = Self::get_or_create_key()?;
        let signature = key.sign(data);
        Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature.to_bytes()))
    }

    /// İmzayı doğrular
    pub fn verify(data: &[u8], signature_b64: &str, public_key_bytes: &[u8]) -> Result<bool> {
        let signature_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature_b64)
            .map_err(|_| anyhow!("Geçersiz imza formatı (base64)"))?;
        
        let signature = Signature::from_bytes(&signature_bytes.try_into().map_err(|_| anyhow!("İmza uzunluğu hatalı"))?);
        let public_key = VerifyingKey::from_bytes(&public_key_bytes.try_into().map_err(|_| anyhow!("Public key uzunluğu hatalı"))?)
            .map_err(|_| anyhow!("Geçersiz public key"))?;

        Ok(public_key.verify(data, &signature).is_ok())
    }
}
