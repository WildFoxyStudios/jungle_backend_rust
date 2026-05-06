pub fn verification_email(code: &str, site_name: &str) -> (String, String) {
    let subject = format!("{} — Verify your email", site_name);
    let body = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:Arial,sans-serif;max-width:600px;margin:0 auto;padding:20px">
  <h2 style="color:#2b5876">Email Verification</h2>
  <p>Your verification code is:</p>
  <div style="background:#f4f4f4;padding:15px 25px;font-size:28px;letter-spacing:6px;font-weight:bold;text-align:center;border-radius:8px;margin:20px 0">
    {code}
  </div>
  <p style="color:#666;font-size:13px">This code expires in 10 minutes. If you did not request this, ignore this email.</p>
  <hr style="border:none;border-top:1px solid #eee;margin:20px 0">
  <p style="color:#999;font-size:12px">&copy; {site_name}</p>
</body></html>"#
    );
    (subject, body)
}

pub fn password_reset_email(reset_link: &str, site_name: &str) -> (String, String) {
    let subject = format!("{} — Reset your password", site_name);
    let body = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:Arial,sans-serif;max-width:600px;margin:0 auto;padding:20px">
  <h2 style="color:#2b5876">Password Reset</h2>
  <p>Click the button below to reset your password:</p>
  <div style="text-align:center;margin:30px 0">
    <a href="{reset_link}" style="background:#2b5876;color:#fff;padding:12px 30px;text-decoration:none;border-radius:6px;font-size:16px">
      Reset Password
    </a>
  </div>
  <p style="color:#666;font-size:13px">This link expires in 1 hour. If you did not request a password reset, ignore this email.</p>
  <p style="color:#999;font-size:11px;word-break:break-all">Direct link: {reset_link}</p>
  <hr style="border:none;border-top:1px solid #eee;margin:20px 0">
  <p style="color:#999;font-size:12px">&copy; {site_name}</p>
</body></html>"#
    );
    (subject, body)
}

pub fn notification_email(
    notification_type: &str,
    sender_name: &str,
    text: &str,
    site_name: &str,
    site_url: &str,
) -> (String, String) {
    let subject = format!("{} — {}", site_name, text);
    let body = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:Arial,sans-serif;max-width:600px;margin:0 auto;padding:20px">
  <h2 style="color:#2b5876">{site_name}</h2>
  <div style="background:#f9f9f9;padding:15px 20px;border-radius:8px;margin:15px 0">
    <p style="margin:0"><strong>{sender_name}</strong></p>
    <p style="margin:5px 0 0;color:#333">{text}</p>
    <p style="margin:5px 0 0;color:#999;font-size:12px">Type: {notification_type}</p>
  </div>
  <div style="text-align:center;margin:25px 0">
    <a href="{site_url}" style="background:#2b5876;color:#fff;padding:10px 25px;text-decoration:none;border-radius:6px;font-size:14px">
      View on {site_name}
    </a>
  </div>
  <hr style="border:none;border-top:1px solid #eee;margin:20px 0">
  <p style="color:#999;font-size:12px">&copy; {site_name}</p>
</body></html>"#
    );
    (subject, body)
}

/// Template for the "unusual sign-in" confirmation email. Surfaces both
/// the approval code and the IP address we saw so the user can decide
/// whether the attempt was legitimate before entering the code.
pub fn unusual_login_email(
    code: &str,
    site_name: &str,
    ip: &str,
    first_name: &str,
) -> (String, String) {
    let subject = format!("{} — Unusual sign-in attempt", site_name);
    let body = format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:Arial,sans-serif;max-width:600px;margin:0 auto;padding:20px">
  <h2 style="color:#c0392b">Unusual sign-in detected</h2>
  <p>Hi {first_name},</p>
  <p>We noticed a sign-in attempt to your {site_name} account from a new device or location:</p>
  <div style="background:#fdecea;padding:10px 15px;border-radius:6px;font-size:14px;margin:10px 0">
    <strong>IP address:</strong> {ip}
  </div>
  <p>If this was you, enter the code below to finish signing in:</p>
  <div style="background:#f4f4f4;padding:15px 25px;font-size:28px;letter-spacing:6px;font-weight:bold;text-align:center;border-radius:8px;margin:20px 0">
    {code}
  </div>
  <p style="color:#666;font-size:13px">This code expires in 10 minutes. If you did not try to sign in, change your password immediately.</p>
  <hr style="border:none;border-top:1px solid #eee;margin:20px 0">
  <p style="color:#999;font-size:12px">&copy; {site_name}</p>
</body></html>"#
    );
    (subject, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_email_contains_code() {
        let (subject, body) = verification_email("123456", "TestSite");
        assert!(subject.contains("TestSite"));
        assert!(body.contains("123456"));
    }

    #[test]
    fn test_unusual_login_email_contains_ip_and_code() {
        let (subject, body) = unusual_login_email("987654", "TestSite", "1.2.3.4", "Alice");
        assert!(subject.contains("Unusual"));
        assert!(body.contains("987654"));
        assert!(body.contains("1.2.3.4"));
        assert!(body.contains("Alice"));
    }

    #[test]
    fn test_password_reset_email_contains_link() {
        let (subject, body) = password_reset_email("https://example.com/reset?t=abc", "TestSite");
        assert!(subject.contains("Reset"));
        assert!(body.contains("https://example.com/reset?t=abc"));
    }

    #[test]
    fn test_notification_email_contains_text() {
        let (subject, body) = notification_email(
            "follow",
            "John",
            "John started following you",
            "TestSite",
            "https://example.com",
        );
        assert!(subject.contains("TestSite"));
        assert!(body.contains("John"));
        assert!(body.contains("John started following you"));
    }
}
