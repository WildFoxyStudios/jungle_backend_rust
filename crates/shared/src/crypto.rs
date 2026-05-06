//! Symmetric encryption for secrets stored in the database (API keys, OAuth secrets,
//! storage credentials, etc.). Uses AES-256-GCM with a random 96-bit nonce.
//!
//! Format of the encoded string: `base64(nonce) || "." || base64(ciphertext)`.
//!
//! The encryption key is derived from a master secret via SHA-256 to always
//! produce a 32-byte key regardless of the input length.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rand::RngCore;
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    Encrypt(String),

    #[error("decryption failed: {0}")]
    Decrypt(String),

    #[error("invalid ciphertext format")]
    InvalidFormat,

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Derive a 32-byte key from an arbitrary master secret.
pub fn derive_key(master: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(master);
    let out = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&out);
    key
}

/// Encrypt `plaintext` using the given 32-byte key.
/// Returns `base64(nonce).base64(ciphertext_with_tag)`.
pub fn encrypt(key: &[u8], plaintext: &str) -> Result<String, CryptoError> {
    let derived = if key.len() == 32 {
        let mut k = [0u8; 32];
        k.copy_from_slice(key);
        k
    } else {
        derive_key(key)
    };

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived));

    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::Encrypt(e.to_string()))?;

    Ok(format!(
        "{}.{}",
        B64.encode(nonce_bytes),
        B64.encode(ciphertext)
    ))
}

/// Decrypt a string produced by [`encrypt`].
pub fn decrypt(key: &[u8], encoded: &str) -> Result<String, CryptoError> {
    let derived = if key.len() == 32 {
        let mut k = [0u8; 32];
        k.copy_from_slice(key);
        k
    } else {
        derive_key(key)
    };

    let (nonce_b64, ct_b64) = encoded.split_once('.').ok_or(CryptoError::InvalidFormat)?;

    let nonce_bytes = B64.decode(nonce_b64)?;
    if nonce_bytes.len() != 12 {
        return Err(CryptoError::InvalidFormat);
    }
    let ciphertext = B64.decode(ct_b64)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|e| CryptoError::Decrypt(e.to_string()))?;

    String::from_utf8(plaintext).map_err(|e| CryptoError::Decrypt(e.to_string()))
}

/// Mask a secret for display (e.g. `sk-p...ijkl`).
pub fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len().max(4));
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{}...{}", head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ok() {
        let key = derive_key(b"my-master-secret");
        let plain = "sk-proj-abc123xyz";
        let ct = encrypt(&key, plain).unwrap();
        assert_ne!(ct, plain);
        assert!(ct.contains('.'));
        let pt = decrypt(&key, &ct).unwrap();
        assert_eq!(pt, plain);
    }

    #[test]
    fn different_nonces_yield_different_ciphertexts() {
        let key = derive_key(b"abcdef");
        let a = encrypt(&key, "hello").unwrap();
        let b = encrypt(&key, "hello").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn decrypt_rejects_tampered_ciphertext() {
        let key = derive_key(b"k");
        let ct = encrypt(&key, "secret").unwrap();
        let tampered = ct.replace(&ct[ct.len() - 4..], "AAAA");
        assert!(decrypt(&key, &tampered).is_err());
    }

    #[test]
    fn mask_short_secret() {
        assert_eq!(mask_secret("abc"), "****");
        assert_eq!(mask_secret("abcdefgh"), "********");
    }

    #[test]
    fn mask_long_secret() {
        assert_eq!(mask_secret("sk-proj-12345678abcdefghijkl"), "sk-p...ijkl");
    }
}
