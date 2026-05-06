//! Cross-provider signature verification helpers.
//!
//! Each payment provider has a slightly different convention for how webhook
//! bodies are signed, but they all boil down to three building blocks:
//!
//! 1. A keyed hash (HMAC-SHA1/256/512 or MD5 with concatenated secret).
//! 2. A hex or base64 encoding of that hash over the raw request body (and/or
//!    selected headers).
//! 3. A constant-time comparison against the signature header sent by the
//!    provider.
//!
//! Implementing each of these inline in every provider module leads to
//! subtle timing leaks and inconsistencies (for example, `==` on hex strings
//! leaks length; case-insensitive hex fails unless normalised first). This
//! module centralises the primitives so every provider gets the same
//! hardening.

use hmac::{Hmac, Mac};
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

use crate::gateway::PaymentError;

/// Constant-time byte-slice equality. Uses a bitwise XOR accumulation that
/// processes all bytes regardless of mismatch position, so attackers cannot
/// probe where an HMAC diverges via timing side-channels.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Compute HMAC-SHA256 of `payload` using `key`, returning lowercase hex.
pub fn hmac_sha256_hex(key: &[u8], payload: &[u8]) -> Result<String, PaymentError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|e| PaymentError::ProviderError(format!("HMAC key error: {e}")))?;
    mac.update(payload);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Compute HMAC-SHA512 of `payload` using `key`, returning lowercase hex.
pub fn hmac_sha512_hex(key: &[u8], payload: &[u8]) -> Result<String, PaymentError> {
    let mut mac = Hmac::<Sha512>::new_from_slice(key)
        .map_err(|e| PaymentError::ProviderError(format!("HMAC key error: {e}")))?;
    mac.update(payload);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Verify a hex-encoded HMAC-SHA256 signature. Accepts signatures with or
/// without a leading algorithm prefix like `sha256=` (Coinbase Commerce,
/// GitHub-style).
pub fn verify_hmac_sha256_hex(
    key: &[u8],
    payload: &[u8],
    provided: &str,
) -> Result<(), PaymentError> {
    if key.is_empty() {
        return Err(PaymentError::ProviderError(
            "Webhook secret not configured".into(),
        ));
    }
    let provided = provided
        .strip_prefix("sha256=")
        .unwrap_or(provided)
        .trim()
        .to_ascii_lowercase();
    if provided.is_empty() {
        return Err(PaymentError::InvalidSignature);
    }
    let expected = hmac_sha256_hex(key, payload)?;
    if !constant_time_eq(expected.as_bytes(), provided.as_bytes()) {
        return Err(PaymentError::InvalidSignature);
    }
    Ok(())
}

/// Verify a hex-encoded HMAC-SHA512 signature (case-insensitive).
pub fn verify_hmac_sha512_hex(
    key: &[u8],
    payload: &[u8],
    provided: &str,
) -> Result<(), PaymentError> {
    if key.is_empty() {
        return Err(PaymentError::ProviderError(
            "Webhook secret not configured".into(),
        ));
    }
    let provided = provided
        .strip_prefix("sha512=")
        .unwrap_or(provided)
        .trim()
        .to_ascii_lowercase();
    if provided.is_empty() {
        return Err(PaymentError::InvalidSignature);
    }
    let expected = hmac_sha512_hex(key, payload)?;
    if !constant_time_eq(expected.as_bytes(), provided.as_bytes()) {
        return Err(PaymentError::InvalidSignature);
    }
    Ok(())
}

/// Verify a Braintree webhook signature.
///
/// Braintree's webhook notifications arrive as `application/x-www-form-urlencoded`
/// with two fields: `bt_signature` and `bt_payload`. The signature is
/// `<public_key>|<hmac_sha1_hex>` where the HMAC key is `SHA1(private_key)`
/// (yes — first SHA1-hash the private key, then use the resulting 20-byte
/// digest as the HMAC key) and the HMAC input is the raw `bt_payload`.
///
/// Reference: braintree_ruby/lib/braintree/webhook_notification_gateway.rb
/// https://github.com/braintree/braintree_ruby/blob/master/lib/braintree/webhook_notification_gateway.rb
pub fn verify_braintree_signature(
    public_key: &str,
    private_key: &str,
    bt_signature: &str,
    bt_payload: &str,
) -> Result<(), PaymentError> {
    if private_key.is_empty() || public_key.is_empty() {
        return Err(PaymentError::ProviderError(
            "Braintree credentials not configured".into(),
        ));
    }
    // Braintree may send multiple key|signature pairs separated by `&`.
    let candidate = bt_signature
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '|');
            let pk = parts.next()?.trim();
            let sig = parts.next()?.trim();
            if pk == public_key { Some(sig) } else { None }
        })
        .next()
        .ok_or(PaymentError::InvalidSignature)?;

    // Key = SHA1(private_key) → 20-byte digest used as HMAC key.
    let mut hasher = Sha1::new();
    hasher.update(private_key.as_bytes());
    let hmac_key = hasher.finalize();

    let mut mac = Hmac::<Sha1>::new_from_slice(&hmac_key)
        .map_err(|e| PaymentError::ProviderError(format!("HMAC key error: {e}")))?;
    mac.update(bt_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    if !constant_time_eq(expected.as_bytes(), candidate.as_bytes()) {
        return Err(PaymentError::InvalidSignature);
    }
    Ok(())
}

/// Verify a raw shared-secret token (e.g. Flutterwave `verif-hash` header)
/// using constant-time equality. Use this when the provider does not sign
/// the body but sends the same static secret on every request.
pub fn verify_shared_secret(configured: &[u8], provided: &str) -> Result<(), PaymentError> {
    if configured.is_empty() {
        return Err(PaymentError::ProviderError(
            "Webhook secret not configured".into(),
        ));
    }
    if provided.is_empty() {
        return Err(PaymentError::InvalidSignature);
    }
    if !constant_time_eq(configured, provided.as_bytes()) {
        return Err(PaymentError::InvalidSignature);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_same() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(constant_time_eq(&[], &[]));
    }

    #[test]
    fn constant_time_eq_diff() {
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"", b"a"));
    }

    #[test]
    fn verify_hmac_sha256_hex_accepts_prefixed() {
        let key = b"secret";
        let payload = b"hello";
        let sig = hmac_sha256_hex(key, payload).unwrap();
        let with_prefix = format!("sha256={sig}");
        assert!(verify_hmac_sha256_hex(key, payload, &with_prefix).is_ok());
    }

    #[test]
    fn verify_hmac_sha256_hex_case_insensitive() {
        let key = b"secret";
        let payload = b"hello";
        let sig = hmac_sha256_hex(key, payload).unwrap().to_uppercase();
        assert!(verify_hmac_sha256_hex(key, payload, &sig).is_ok());
    }

    #[test]
    fn verify_hmac_sha256_hex_rejects_wrong_sig() {
        let result = verify_hmac_sha256_hex(b"secret", b"hello", "0000");
        assert!(matches!(result, Err(PaymentError::InvalidSignature)));
    }

    #[test]
    fn verify_hmac_sha256_hex_rejects_missing_secret() {
        let result = verify_hmac_sha256_hex(b"", b"hello", "abcd");
        assert!(matches!(result, Err(PaymentError::ProviderError(_))));
    }

    #[test]
    fn verify_shared_secret_ok() {
        assert!(verify_shared_secret(b"secret", "secret").is_ok());
    }

    #[test]
    fn verify_shared_secret_wrong() {
        let r = verify_shared_secret(b"secret", "wrong");
        assert!(matches!(r, Err(PaymentError::InvalidSignature)));
    }

    #[test]
    fn verify_braintree_signature_roundtrip() {
        // Generate the expected signature with the exact algorithm the
        // Braintree SDK uses, then feed it back to verify.
        let public_key = "pubkey";
        let private_key = "privkey";
        let payload = "<notification>hello</notification>";

        let mut hasher = Sha1::new();
        hasher.update(private_key.as_bytes());
        let hmac_key = hasher.finalize();
        let mut mac = Hmac::<Sha1>::new_from_slice(&hmac_key).unwrap();
        mac.update(payload.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let bt_signature = format!("{public_key}|{sig}");

        verify_braintree_signature(public_key, private_key, &bt_signature, payload).unwrap();
    }

    #[test]
    fn verify_braintree_signature_rejects_wrong() {
        let r = verify_braintree_signature("pub", "priv", "pub|deadbeef", "payload");
        assert!(matches!(r, Err(PaymentError::InvalidSignature)));
    }

    #[test]
    fn verify_braintree_signature_rejects_public_key_mismatch() {
        let r = verify_braintree_signature("pub", "priv", "other|deadbeef", "payload");
        assert!(matches!(r, Err(PaymentError::InvalidSignature)));
    }
}
