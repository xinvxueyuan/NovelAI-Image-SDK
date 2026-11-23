//! Client configuration.
//!
//! Port of the core fields of `novelai_image_mcp.settings.NovelAISettings`.
//! The SDK never reads environment variables itself — language bindings load
//! `NOVELAI_*` variables per their own conventions and pass a [`ClientConfig`].

use std::time::Duration;

use crate::auth::Credentials;
use crate::browser::BrowserProfile;
use crate::constants::Model;
use crate::error::Error;

/// Default timeout in seconds, matching the Python client.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Default vibe-cache capacity (`reference` → encoded token).
pub const DEFAULT_VIBE_CACHE_ENTRIES: usize = 64;

/// `image.novelai.net`: the consolidated host for most third-party traffic
/// (generation, director, account, upscaling).
pub const DEFAULT_IMAGE_BASE_URL: &str = "https://image.novelai.net";

/// The Primary API host; still the home of `/ai/annotate-image`, the one
/// endpoint that was never migrated.
pub const DEFAULT_LEGACY_IMAGE_BASE_URL: &str = "https://api.novelai.net";

/// Configuration for a NovelAI client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Credentials: a bearer token, or a username/password pair.
    pub credentials: Credentials,
    /// Host for `/ai/*` image endpoints and `/user/*`.
    pub image_base_url: String,
    /// Host for `/user/login`.
    pub account_base_url: String,
    /// Host for the un-migrated `/ai/annotate-image`.
    pub legacy_image_base_url: String,
    /// Per-request timeout for unary calls.
    ///
    /// Streams use an idle-chunk timeout instead, because a total deadline
    /// would kill long generations.
    pub timeout: Duration,
    /// Vibe-transfer cache capacity.
    pub vibe_cache_entries: usize,
    /// Browser fingerprint profile.
    pub profile: BrowserProfile,
    /// The model used when a request does not specify one.
    pub default_model: Model,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            credentials: Credentials::default(),
            image_base_url: DEFAULT_IMAGE_BASE_URL.to_owned(),
            account_base_url: DEFAULT_IMAGE_BASE_URL.to_owned(),
            legacy_image_base_url: DEFAULT_LEGACY_IMAGE_BASE_URL.to_owned(),
            timeout: DEFAULT_TIMEOUT,
            vibe_cache_entries: DEFAULT_VIBE_CACHE_ENTRIES,
            profile: BrowserProfile::chrome(),
            default_model: Model::V4_5,
        }
    }
}

impl ClientConfig {
    /// A config with the given credentials and documented defaults.
    pub fn new(credentials: Credentials) -> Self {
        Self {
            credentials,
            ..Self::default()
        }
    }

    /// Override the image host (useful for tests).
    pub fn with_image_base_url(mut self, url: impl Into<String>) -> Self {
        self.image_base_url = url.into();
        self
    }

    /// Override the account host (useful for tests).
    pub fn with_account_base_url(mut self, url: impl Into<String>) -> Self {
        self.account_base_url = url.into();
        self
    }

    /// Apply caller-supplied overrides; `None` keeps the default.
    ///
    /// The FFI boundaries use this so the 44-field/option mapping exists once.
    pub fn apply_overrides(&mut self, overrides: ConfigOverrides) {
        if let Some(value) = overrides.image_base_url {
            self.image_base_url = value;
        }
        if let Some(value) = overrides.account_base_url {
            self.account_base_url = value;
        }
        if let Some(value) = overrides.legacy_image_base_url {
            self.legacy_image_base_url = value;
        }
        if let Some(value) = overrides.timeout {
            self.timeout = value;
        }
        if let Some(value) = overrides.vibe_cache_entries {
            self.vibe_cache_entries = value;
        }
    }

    /// Validate the configuration (credentials present, URLs well-formed).
    pub fn validate(&self) -> Result<(), Error> {
        self.credentials.validate()?;
        for (name, url) in [
            ("image_base_url", &self.image_base_url),
            ("account_base_url", &self.account_base_url),
            ("legacy_image_base_url", &self.legacy_image_base_url),
        ] {
            validate_base_url(name, url)?;
        }
        if self.vibe_cache_entries == 0 {
            return Err(Error::config("vibe_cache_entries must be positive"));
        }
        Ok(())
    }
}

/// Optional config overrides, shared by the FFI boundaries.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    /// Image host.
    pub image_base_url: Option<String>,
    /// Account host.
    pub account_base_url: Option<String>,
    /// Legacy host for `/ai/annotate-image`.
    pub legacy_image_base_url: Option<String>,
    /// Per-request timeout.
    pub timeout: Option<std::time::Duration>,
    /// Vibe cache capacity.
    pub vibe_cache_entries: Option<usize>,
}

/// A base URL must be an absolute http(s) URL.
fn validate_base_url(name: &str, url: &str) -> Result<(), Error> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return Ok(());
    }
    Err(Error::config(format!("{name} must be an http(s) URL")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_python() {
        let config = ClientConfig::default();
        assert_eq!(config.image_base_url, "https://image.novelai.net");
        assert_eq!(config.account_base_url, "https://image.novelai.net");
        assert_eq!(config.legacy_image_base_url, "https://api.novelai.net");
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.vibe_cache_entries, 64);
        assert_eq!(config.default_model, Model::V4_5);
    }

    #[test]
    fn validation_rejects_missing_credentials() {
        assert!(ClientConfig::default().validate().is_err());
        assert!(
            ClientConfig::new(Credentials::token("t"))
                .validate()
                .is_ok()
        );
    }
}
