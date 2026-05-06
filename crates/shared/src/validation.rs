use once_cell::sync::Lazy;
use regex::Regex;
use validator::ValidationError;

static USERNAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]{3,32}$").unwrap());

pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    if !USERNAME_REGEX.is_match(username) {
        let mut err = ValidationError::new("invalid_username");
        err.message =
            Some("Username must be 3-32 characters, alphanumeric and underscores only".into());
        return Err(err);
    }
    Ok(())
}

pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        let mut err = ValidationError::new("weak_password");
        err.message = Some("Password must be at least 8 characters".into());
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_username() {
        assert!(validate_username("john_doe").is_ok());
        assert!(validate_username("User123").is_ok());
        assert!(validate_username("abc").is_ok());
    }

    #[test]
    fn test_invalid_username_too_short() {
        assert!(validate_username("ab").is_err());
    }

    #[test]
    fn test_invalid_username_special_chars() {
        assert!(validate_username("john doe").is_err());
        assert!(validate_username("john@doe").is_err());
        assert!(validate_username("john-doe").is_err());
    }

    #[test]
    fn test_password_strength_valid() {
        assert!(validate_password_strength("password123").is_ok());
        assert!(validate_password_strength("12345678").is_ok());
    }

    #[test]
    fn test_password_strength_too_short() {
        assert!(validate_password_strength("1234567").is_err());
        assert!(validate_password_strength("abc").is_err());
    }
}
