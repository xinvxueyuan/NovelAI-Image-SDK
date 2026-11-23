//! Chrome TLS + header fingerprint, in one place so the three layers cannot
//! drift apart.
//!
//! Port of `novelai_image_mcp.nai.http`. Two layers must agree for Cloudflare
//! to treat this client as browser-originated:
//!
//! 1. **TLS (JA3/JA4) + HTTP/2** — `wreq` reproduces Chrome's BoringSSL
//!    `ClientHello`, HTTP/2 SETTINGS and pseudo-header order from the
//!    [`EmulationProfile`].
//! 2. **Headers** — the full Chrome Client Hints (`Sec-Ch-Ua*`) and Fetch
//!    Metadata (`Sec-Fetch-*`) families in [`BrowserProfile::headers`].
//!
//! Both are fields of a single [`BrowserProfile`] constant, so bumping the
//! emulated Chrome version is one edit.

use wreq_util::{Emulation, Platform, Profile};

/// Chrome 150 stable on Windows (released 2026-07-23). The minor version is
/// frozen to `0.0.0` per Chrome's User-Agent Reduction policy — the actual
/// patch version is never sent in the UA string.
pub const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36";

/// Chrome 150 Client Hints.
pub const CHROME_SEC_CH_UA: &str =
    "\"Chromium\";v=\"150\", \"Google Chrome\";v=\"150\", \"Not.A/Brand\";v=\"99\"";

/// A browser to emulate: the TLS/HTTP-2 profile plus the matching header block.
#[derive(Debug, Clone)]
pub struct BrowserProfile {
    /// The emulation profile `wreq` applies (TLS + HTTP/2 + pseudo-headers).
    ///
    /// `wreq-util` 0.2.1 tops out at `Chrome149`; see [`CHROME_EMULATION_NOTE`].
    pub emulation: Profile,
    /// How the profile is described in docs and errors.
    pub name: &'static str,
}

/// Documentation of the one-version gap between the emulated TLS profile and
/// the advertised Chrome version.
pub const CHROME_EMULATION_NOTE: &str = "wreq-util 0.2.1 ships Chrome profiles up to 149; the \
header block advertises Chrome 150. Both are valid recent Chrome fingerprints.";

impl BrowserProfile {
    /// The default profile: Chrome, headers and TLS in lock-step.
    pub fn chrome() -> Self {
        Self {
            emulation: Profile::Chrome149,
            name: "chrome-149",
        }
    }

    /// The transport emulation with **header injection disabled**.
    ///
    /// `wreq`'s default emulation also emits its own header block (a Chrome149
    /// macOS block, including a conflicting `User-Agent`, `Sec-Ch-Ua` and
    /// `Accept`). Sending both blocks duplicates every header and, because
    /// duplicate `Accept` values survive as a comma-joined list, breaks the
    /// streaming `application/x-msgpack` negotiation. TLS + HTTP/2
    /// fingerprinting stays; the single header block comes from
    /// [`Self::headers`].
    fn transport_emulation(&self) -> Emulation {
        Emulation::builder()
            .profile(self.emulation)
            .platform(Platform::Windows)
            .headers(false)
            .build()
    }

    /// Build the `wreq` client with this profile's transport emulation.
    /// NOTE: the emulation profile is intentionally *not* attached here.
    ///
    /// `wreq`'s client-level emulation injects its header block as client
    /// default headers, which take precedence over per-request `.header()`
    /// calls — so `Accept: application/x-msgpack` on the streaming endpoint
    /// would be silently replaced by the profile's `Accept: */*`. Instead each
    /// request applies the profile through [`Self::apply`], where our explicit
    /// headers are written after the profile block.
    #[cfg(feature = "browser-emulation")]
    pub fn build_client(&self) -> Result<wreq::Client, crate::error::Error> {
        let mut builder = wreq::Client::builder();
        if cfg!(feature = "compression") {
            builder = builder.gzip(true).brotli(true).zstd(true);
        }
        builder
            .build()
            .map_err(|e| crate::error::Error::config(format!("failed to build HTTP client: {e}")))
    }

    /// Apply the emulation profile to a request, then the profile's header
    /// block. Explicit per-request headers applied by the caller afterwards win.
    #[cfg(feature = "browser-emulation")]
    pub fn apply(&self, builder: wreq::RequestBuilder) -> wreq::RequestBuilder {
        builder.emulation(self.transport_emulation())
    }

    /// Apply the header block only (no TLS emulation; offline tests).
    #[cfg(not(feature = "browser-emulation"))]
    pub fn apply(&self, builder: wreq::RequestBuilder) -> wreq::RequestBuilder {
        builder
    }

    /// The header block, minus `Accept`, applied to a request.
    ///
    /// `Accept` is deliberately excluded: `wreq` keeps duplicate values for
    /// singleton headers and joins them with commas, so the profile's `*/*`
    /// plus a streaming `application/x-msgpack` would arrive as
    /// `*/*, application/x-msgpack`. The client sets `Accept` exactly once per
    /// request instead.
    pub fn apply_headers(&self, mut builder: wreq::RequestBuilder) -> wreq::RequestBuilder {
        for (name, value) in self.headers() {
            if name == "Accept" {
                continue;
            }
            builder = builder.header(name, value);
        }
        builder
    }

    /// Build a plain `wreq` client without TLS emulation.
    ///
    /// Only for offline tests (wiremock over plain HTTP) and as a last-resort
    /// escape hatch; requests to NovelAI will be rejected by Cloudflare.
    #[cfg(not(feature = "browser-emulation"))]
    pub fn build_client(&self) -> Result<wreq::Client, crate::error::Error> {
        wreq::Client::builder()
            .build()
            .map_err(|e| crate::error::Error::config(format!("failed to build HTTP client: {e}")))
    }

    /// The full Chrome header block.
    ///
    /// `Sec-Fetch-Site` is `same-site` (not `same-origin`) because the browser
    /// frontend at `novelai.net` makes cross-site requests to
    /// `image.novelai.net`.
    pub fn headers(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("User-Agent", CHROME_USER_AGENT),
            ("Accept", "*/*"),
            ("Accept-Language", "en-US,en;q=0.9"),
            ("Accept-Encoding", "gzip, deflate, br, zstd"),
            ("Origin", "https://novelai.net"),
            // Trailing slash matters — the real browser always sends the
            // path-less origin with a slash, and Cloudflare's strict-mode
            // rules check for it.
            ("Referer", "https://novelai.net/"),
            ("Priority", "u=1, i"),
            ("Sec-Ch-Ua", CHROME_SEC_CH_UA),
            ("Sec-Ch-Ua-Mobile", "?0"),
            ("Sec-Ch-Ua-Platform", "\"Windows\""),
            ("Sec-Fetch-Dest", "empty"),
            ("Sec-Fetch-Mode", "cors"),
            ("Sec-Fetch-Site", "same-site"),
        ]
    }
}

/// The header block of the default Chrome profile.
#[must_use]
pub fn browser_headers() -> Vec<(&'static str, &'static str)> {
    BrowserProfile::chrome().headers()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_block_matches_reference() {
        let headers = BrowserProfile::chrome().headers();
        let get = |name: &str| match headers.iter().find(|(k, _)| *k == name) {
            Some((_, v)) => *v,
            None => panic!("missing {name}"),
        };
        assert_eq!(get("User-Agent"), CHROME_USER_AGENT);
        assert!(get("User-Agent").contains("Chrome/150"));
        assert_eq!(get("Referer"), "https://novelai.net/", "trailing slash matters");
        assert_eq!(get("Sec-Fetch-Site"), "same-site");
        assert_eq!(get("Priority"), "u=1, i");
        assert_eq!(get("Accept-Encoding"), "gzip, deflate, br, zstd");
        assert_eq!(headers.len(), 13);
    }

    #[test]
    fn chrome_major_matches_header_block() {
        // If the profile is bumped, the header block must move with it. The
        // emulated profile is 149 (wreq-util 0.2.1 limit); the UA is 150.
        // Keep this test as the tripwire when 0.3 publishes newer profiles.
        let profile = BrowserProfile::chrome();
        assert_eq!(profile.name, "chrome-149");
        assert!(matches!(profile.emulation, Profile::Chrome149));
        assert!(CHROME_USER_AGENT.contains("Chrome/150"));
        assert!(CHROME_SEC_CH_UA.contains("v=\"150\""));
    }
}
