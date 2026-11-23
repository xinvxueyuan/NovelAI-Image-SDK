//! NovelAI credential validation, access-key derivation and tracking headers.
//!
//! Port of `novelai_image_mcp.nai.auth`.

use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use blake2::Digest;
use rand::Rng;
use time::OffsetDateTime;

use crate::error::Error;

/// NovelAI credentials: either a pre-issued bearer token or a
/// username/password pair from which the login key is derived.
///
/// Construct with [`Credentials::token`] or [`Credentials::password`]; a
/// partial username/password pair is rejected.
#[derive(Debug, Clone, Default)]
pub struct Credentials {
    token: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl Credentials {
    /// Credentials from a NovelAI access token (the common case).
    pub fn token(token: impl Into<String>) -> Self {
        let token = token.into();
        Self {
            token: (!token.trim().is_empty()).then_some(token),
            username: None,
            password: None,
        }
    }

    /// Credentials from a username/password pair; the access key is derived
    /// on first use (Argon2id — deliberately expensive).
    pub fn password(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            token: None,
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    /// The pre-issued token, when configured.
    pub fn token_value(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// The username, when configured.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// The password, when configured.
    #[doc(hidden)]
    pub fn password_value(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Validate the combination, mirroring the Python dataclass invariant:
    /// "provide a token or a complete username/password pair".
    pub fn validate(&self) -> Result<(), Error> {
        let has_username = self.username.as_deref().is_some_and(|u| !u.is_empty());
        let has_password = self.password.as_deref().is_some_and(|p| !p.is_empty());
        let has_pair = has_username && has_password;
        let has_partial_pair = has_username != has_password;
        if has_partial_pair || !(self.token.is_some() || has_pair) {
            return Err(Error::Validation {
                message: "provide a token or a complete username/password pair".into(),
                code: None,
                explanation: None,
            });
        }
        Ok(())
    }

    /// True when any credential material is present.
    pub fn is_configured(&self) -> bool {
        self.token.is_some() || (self.username.is_some() && self.password.is_some())
    }
}

impl fmt::Display for Credentials {
    /// Never prints credential material.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = if self.token.is_some() {
            "token"
        } else if self.username.is_some() {
            "password"
        } else {
            "empty"
        };
        write!(f, "Credentials({kind})")
    }
}

/// The exact Argon2id parameters NovelAI's web client uses.
///
/// Construction is mandated by the `novelai_data_access_key` protocol;
/// altering any parameter breaks upstream compatibility.
const ARGON2_TIME_COST: u32 = 2;
const ARGON2_MEMORY_KIB: u32 = 1_953;
const ARGON2_PARALLELISM: u32 = 1;
const ARGON2_HASH_LEN: usize = 64;

/// Derive the 64-character NovelAI login key from username/password.
///
/// Argon2id over the password, with a 16-byte blake2b-derived salt built from
/// the password's first six characters, the username and the constant
/// `novelai_data_access_key`. Result is base64url, truncated to 64 chars.
pub fn derive_access_key(username: &str, password: &str) -> Result<String, Error> {
    if username.is_empty() || password.is_empty() {
        return Err(Error::validation(
            "username/password credentials are required",
        ));
    }

    // blake2b derives a 16-byte deterministic salt for the argon2id call below;
    // it is NOT the password hash.
    let salt_source = format!(
        "{}{username}novelai_data_access_key",
        password.chars().take(6).collect::<String>()
    );
    let salt = blake2::Blake2b::<blake2::digest::consts::U16>::digest(salt_source.as_bytes());

    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(ARGON2_HASH_LEN),
    )
    .map_err(|e| Error::config(format!("invalid Argon2 parameters: {e}")))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut derived = [0u8; ARGON2_HASH_LEN];
    argon2
        .hash_password_into(password.as_bytes(), &salt, &mut derived)
        .map_err(|e| Error::config(format!("Argon2 derivation failed: {e}")))?;

    let encoded = base64::engine::general_purpose::URL_SAFE.encode(derived);
    Ok(encoded.chars().take(64).collect())
}

/// Derive the login key from a [`Credentials`] pair.
pub fn access_key(credentials: &Credentials) -> Result<String, Error> {
    match (credentials.username(), credentials.password_value()) {
        (Some(username), Some(password)) => derive_access_key(username, password),
        _ => Err(Error::validation(
            "username/password credentials are required",
        )),
    }
}

/// Fresh tracking headers NovelAI's browser-facing APIs expect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackingHeaders {
    /// `x-correlation-id`
    pub correlation_id: String,
    /// `x-initiated-at` (ISO-8601, millisecond precision, UTC, `Z` suffix)
    pub initiated_at: String,
}

impl TrackingHeaders {
    /// Build fresh headers, regenerated per request.
    pub fn generate() -> Self {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::rng();
        let correlation_id = (0..6)
            .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
            .collect();

        // ISO-8601 with millisecond precision: 2026-11-23T12:34:56.789Z
        let now = OffsetDateTime::now_utc();
        let initiated_at = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            now.year(),
            u8::from(now.month()),
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            now.millisecond(),
        );

        Self {
            correlation_id,
            initiated_at,
        }
    }

    /// Iterate as `(name, value)` header pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            ("x-correlation-id", self.correlation_id.as_str()),
            ("x-initiated-at", self.initiated_at.as_str()),
        ]
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_validation_matches_python_invariant() {
        assert!(Credentials::token("tok").validate().is_ok());
        assert!(Credentials::password("u", "p").validate().is_ok());
        assert!(Credentials::default().validate().is_err());

        let partial = Credentials {
            token: None,
            username: Some("u".into()),
            password: None,
        };
        assert!(partial.validate().is_err());
        assert!(Credentials::token("").validate().is_err());
    }

    #[test]
    fn tracking_headers_shape() {
        let headers = TrackingHeaders::generate();
        assert_eq!(headers.correlation_id.len(), 6);
        assert!(headers.correlation_id.bytes().all(|b| b.is_ascii_alphanumeric()));
        // 2026-11-23T12:34:56.789Z
        let ts = &headers.initiated_at;
        assert_eq!(ts.len(), 24, "{ts}");
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[19..20], ".");
    }

    #[test]
    fn access_key_is_64_urlsafe_chars() {
        // Deterministic: the derivation has no randomness.
        let key = derive_access_key("user@example.com", "hunter2").unwrap();
        assert_eq!(key.len(), 64, "{key}");
        assert!(key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
        assert_eq!(key, derive_access_key("user@example.com", "hunter2").unwrap());
    }

    #[test]
    fn credentials_display_never_leaks() {
        let text = Credentials::password("alice", "s3cret").to_string();
        assert!(!text.contains("s3cret"));
        assert!(!text.contains("alice"));
        assert_eq!(text, "Credentials(password)");
    }
}
