//! Agora RTC AccessToken v006 builder.
//!
//! Produces tokens compatible with the Agora SDK using HMAC-SHA256 over the
//! official message format. Matches Agora's official Python/Go/Java reference
//! implementations (see `RtcTokenBuilder.py`).
//!
//! Token layout:
//! ```text
//!   "006" + appId + base64( signature_len(u16) | signature | crc32(chan) | crc32(uid) | msg_len(u16) | msg )
//! where msg = salt(u32) | ts(u32) | n_privs(u16) | [ priv(u16) | expire(u32) ]...
//! ```

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum Privilege {
    JoinChannel = 1,
    PublishAudioStream = 2,
    PublishVideoStream = 3,
    PublishDataStream = 4,
    // (6..=8 reserved for admin capabilities — RtcTokenBuilder does not use them)
}

#[derive(Debug, Clone, Copy)]
pub enum Role {
    /// Publisher = can send audio/video. Required for calling participants.
    Publisher,
    /// Subscriber = can only receive. Used for live-viewer accounts.
    Subscriber,
}

/// Generate an Agora RTC AccessToken v006.
///
/// - `app_id`: the Agora App ID (exactly 32 chars of hex).
/// - `app_certificate`: the secret App Certificate from Agora console.
/// - `channel_name`: the channel user will join (UTF-8).
/// - `uid`: the user id (0 means "any").
/// - `role`: publisher or subscriber.
/// - `expire_secs`: token lifetime in seconds from now.
pub fn build(
    app_id: &str,
    app_certificate: &str,
    channel_name: &str,
    uid: u32,
    role: Role,
    expire_secs: u32,
) -> Result<String, String> {
    if app_id.is_empty() || app_certificate.is_empty() {
        return Err("app_id and app_certificate required".into());
    }

    let uid_str = if uid == 0 {
        String::new()
    } else {
        uid.to_string()
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as u32;
    let privilege_expire_ts = now.saturating_add(expire_secs);

    let mut privileges: Vec<(u16, u32)> = Vec::new();
    privileges.push((Privilege::JoinChannel as u16, privilege_expire_ts));
    if matches!(role, Role::Publisher) {
        privileges.push((Privilege::PublishAudioStream as u16, privilege_expire_ts));
        privileges.push((Privilege::PublishVideoStream as u16, privilege_expire_ts));
        privileges.push((Privilege::PublishDataStream as u16, privilege_expire_ts));
    }

    // message = salt | ts | n_privs | priv_pairs
    let salt: u32 = rand::rng().random_range(1..=99_999_999);
    let ts: u32 = now + 24 * 3600; // per Agora spec, issuance timestamp

    let mut message: Vec<u8> = Vec::new();
    message.extend_from_slice(&salt.to_le_bytes());
    message.extend_from_slice(&ts.to_le_bytes());
    message.extend_from_slice(&(privileges.len() as u16).to_le_bytes());
    for (priv_id, exp) in &privileges {
        message.extend_from_slice(&priv_id.to_le_bytes());
        message.extend_from_slice(&exp.to_le_bytes());
    }

    // Compose the HMAC input: appId | channelName | uidStr | message
    let mut mac_input: Vec<u8> = Vec::new();
    mac_input.extend_from_slice(app_id.as_bytes());
    mac_input.extend_from_slice(channel_name.as_bytes());
    mac_input.extend_from_slice(uid_str.as_bytes());
    mac_input.extend_from_slice(&message);

    let mut mac = <HmacSha256 as Mac>::new_from_slice(app_certificate.as_bytes())
        .map_err(|e| e.to_string())?;
    mac.update(&mac_input);
    let signature = mac.finalize().into_bytes();

    // CRC32 of channel and uid (using IEEE polynomial, little-endian)
    let crc_channel = crc32fast::hash(channel_name.as_bytes());
    let crc_uid = crc32fast::hash(uid_str.as_bytes());

    // content = sig_len(u16) | sig | crc_channel | crc_uid | msg_len(u16) | msg
    let mut content: Vec<u8> = Vec::new();
    content.extend_from_slice(&(signature.len() as u16).to_le_bytes());
    content.extend_from_slice(&signature);
    content.extend_from_slice(&crc_channel.to_le_bytes());
    content.extend_from_slice(&crc_uid.to_le_bytes());
    content.extend_from_slice(&(message.len() as u16).to_le_bytes());
    content.extend_from_slice(&message);

    // token = "006" + appId + base64(content)
    Ok(format!("006{}{}", app_id, B64.encode(&content)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_starts_with_version_and_app_id() {
        let app_id = "aabbccddeeff11223344556677889900";
        let token = build(app_id, "cert", "my-channel", 42, Role::Publisher, 3600).unwrap();
        assert!(
            token.starts_with("006"),
            "token must start with version prefix"
        );
        assert!(
            token[3..].starts_with(app_id),
            "token must embed app_id after version"
        );
        assert!(token.len() > 3 + app_id.len() + 10, "content must follow");
    }

    #[test]
    fn different_salts_produce_different_tokens() {
        let a = build("aaa", "cert", "chan", 1, Role::Publisher, 60).unwrap();
        let b = build("aaa", "cert", "chan", 1, Role::Publisher, 60).unwrap();
        assert_ne!(a, b, "random salt should make tokens differ");
    }

    #[test]
    fn rejects_empty_inputs() {
        assert!(build("", "cert", "chan", 1, Role::Publisher, 60).is_err());
        assert!(build("app", "", "chan", 1, Role::Publisher, 60).is_err());
    }

    #[test]
    fn uid_zero_uses_empty_string() {
        // Just ensure it builds successfully (the byte layout differs for uid=0).
        let t = build("app", "cert", "chan", 0, Role::Publisher, 60).unwrap();
        assert!(!t.is_empty());
    }

    #[test]
    fn subscriber_token_differs_from_publisher() {
        // Subscriber role embeds only the JoinChannel privilege (1 pair) while
        // Publisher embeds 4 (JoinChannel + PublishAudio/Video/Data). The message
        // payload size differs, so tokens must differ even with identical salts.
        let pub_tok = build(
            "aabbccddeeff11223344556677889900",
            "cert",
            "c",
            42,
            Role::Publisher,
            3600,
        )
        .unwrap();
        let sub_tok = build(
            "aabbccddeeff11223344556677889900",
            "cert",
            "c",
            42,
            Role::Subscriber,
            3600,
        )
        .unwrap();
        assert_ne!(
            pub_tok, sub_tok,
            "publisher and subscriber tokens must differ"
        );
        // The subscriber token must still start with the version + app_id prefix.
        assert!(sub_tok.starts_with("006aabbccddeeff11223344556677889900"));
    }
}
