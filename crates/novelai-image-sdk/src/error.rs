//! Error taxonomy, error-body parsing and official code explanations.
//!
//! Direct port of `novelai_image_mcp.nai.exceptions` and `...nai.errors`.
//!
//! Every error optionally carries NovelAI's error [`ErrorCode`] (an HTTP status
//! or an API-supplied stream code) and, when documented, NovelAI's official
//! explanation. The code and explanation are **formatted into the message** —
//! exactly like the Python implementation — so `Display` output matches the
//! MCP server byte-for-byte, and remain available as structured fields for
//! programmatic handling.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A NovelAI error code: an HTTP status or an API-supplied string code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ErrorCode {
    /// An HTTP status code.
    Http(u16),
    /// An API-supplied code (for example a MessagePack stream error code).
    Api(String),
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(status) => write!(f, "{status}"),
            Self::Api(code) => f.write_str(code),
        }
    }
}

/// Official error codes documented by NovelAI's API docs
/// (<https://api.novelai.net/docs>) and the official FAQ.
///
/// 5xx are deliberately folded into the generic provider-error bucket rather
/// than enumerated.
pub const OFFICIAL_CODE_EXPLANATIONS: &[(u16, &str)] = &[
    (400, "invalid request (validation failed)"),
    (
        401,
        "authentication failed (missing, invalid, or expired credentials)",
    ),
    (402, "active subscription or Anlas required"),
    (409, "request conflicted with server state"),
    (429, "rate limit or concurrency limit reached"),
];

/// Return NovelAI's official explanation for a documented error code.
///
/// Undocumented codes (including API-supplied MessagePack stream codes) return
/// [`None`] — the API's own message is then the best explanation.
pub fn explain(code: Option<&ErrorCode>) -> Option<&'static str> {
    match code {
        Some(ErrorCode::Http(status)) => OFFICIAL_CODE_EXPLANATIONS
            .iter()
            .find(|(c, _)| c == status)
            .map(|(_, text)| *text),
        _ => None,
    }
}

/// Format the ` [code X: explanation]` suffix the Python errors append.
fn message_with_suffix(
    message: impl fmt::Display,
    code: Option<&ErrorCode>,
    explanation: Option<&str>,
) -> String {
    match code {
        None => message.to_string(),
        Some(code) => {
            let mut out = format!("{message} [code {code}");
            if let Some(explanation) = explanation.filter(|e| !e.is_empty()) {
                out.push_str(": ");
                out.push_str(explanation);
            }
            out.push(']');
            out
        }
    }
}

/// A NovelAI domain failure.
///
/// The taxonomy mirrors the Python exception hierarchy so message text and
/// classification survive the port:
///
/// ```text
/// Error
/// ├── Provider                     (NovelAI rejected or failed the request)
/// │   ├── Validation               (HTTP 400)
/// │   ├── Authentication           (HTTP 401; also MissingToken)
/// │   ├── InsufficientCredits      (HTTP 402)
/// │   └── Concurrency              (HTTP 429)
/// ├── Timeout
/// ├── Transport
/// ├── Response
/// ├── Image
/// ├── MissingToken
/// └── Config
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The request is invalid (local validation or HTTP 400).
    #[error("{}", message_with_suffix(message, code.as_ref(), explanation.as_deref()))]
    Validation {
        /// Human-readable message (without the code suffix).
        message: String,
        /// NovelAI error code when known.
        code: Option<ErrorCode>,
        /// Official explanation for documented codes.
        explanation: Option<String>,
    },

    /// Credentials are missing, invalid, or expired (HTTP 401).
    #[error("{}", message_with_suffix(message, code.as_ref(), explanation.as_deref()))]
    Authentication {
        /// Human-readable message (without the code suffix).
        message: String,
        /// NovelAI error code when known.
        code: Option<ErrorCode>,
        /// Official explanation for documented codes.
        explanation: Option<String>,
    },

    /// No credentials were configured for the client.
    #[error("{message}")]
    MissingToken {
        /// Human-readable message.
        message: String,
    },

    /// The account has insufficient Anlas or no subscription (HTTP 402).
    #[error("{}", message_with_suffix(message, code.as_ref(), explanation.as_deref()))]
    InsufficientCredits {
        /// Human-readable message (without the code suffix).
        message: String,
        /// NovelAI error code when known.
        code: Option<ErrorCode>,
        /// Official explanation for documented codes.
        explanation: Option<String>,
    },

    /// The account hit a concurrency or rate limit (HTTP 429).
    #[error("{}", message_with_suffix(message, code.as_ref(), explanation.as_deref()))]
    Concurrency {
        /// Human-readable message (without the code suffix).
        message: String,
        /// NovelAI error code when known.
        code: Option<ErrorCode>,
        /// Official explanation for documented codes.
        explanation: Option<String>,
    },

    /// NovelAI rejected or failed the request with any other status.
    #[error("{}", message_with_suffix(message, code.as_ref(), explanation.as_deref()))]
    Provider {
        /// HTTP status when the failure came from a response.
        status: Option<u16>,
        /// Human-readable message (without the code suffix).
        message: String,
        /// NovelAI error code when known.
        code: Option<ErrorCode>,
        /// Official explanation for documented codes.
        explanation: Option<String>,
    },

    /// A request exceeded its timeout.
    #[error("{message}")]
    Timeout {
        /// Human-readable message.
        message: String,
    },

    /// The request could not reach NovelAI (DNS, TLS, connection reset, ...).
    #[error("{message}")]
    Transport {
        /// Human-readable message.
        message: String,
    },

    /// NovelAI returned malformed or unsupported data.
    #[error("{message}")]
    Response {
        /// Human-readable message.
        message: String,
    },

    /// An input or output image is malformed or unsupported.
    #[error("{message}")]
    Image {
        /// Human-readable message.
        message: String,
    },

    /// The SDK or client was misconfigured.
    #[error("{message}")]
    Config {
        /// Human-readable message.
        message: String,
    },
}

impl Error {
    /// A validation error with no NovelAI code.
    pub(crate) fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            code: None,
            explanation: None,
        }
    }

    /// A response-shape error (malformed or unsupported payload).
    pub(crate) fn response(message: impl Into<String>) -> Self {
        Self::Response {
            message: message.into(),
        }
    }

    /// An image error.
    pub(crate) fn image(message: impl Into<String>) -> Self {
        Self::Image {
            message: message.into(),
        }
    }

    /// A configuration error.
    pub(crate) fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// The NovelAI error code, when known.
    pub fn code(&self) -> Option<&ErrorCode> {
        match self {
            Self::Validation { code, .. }
            | Self::Authentication { code, .. }
            | Self::InsufficientCredits { code, .. }
            | Self::Concurrency { code, .. }
            | Self::Provider { code, .. } => code.as_ref(),
            _ => None,
        }
    }

    /// NovelAI's official explanation, when documented.
    pub fn explanation(&self) -> Option<&str> {
        match self {
            Self::Validation { explanation, .. }
            | Self::Authentication { explanation, .. }
            | Self::InsufficientCredits { explanation, .. }
            | Self::Concurrency { explanation, .. }
            | Self::Provider { explanation, .. } => explanation.as_deref(),
            _ => None,
        }
    }
}

/// Normalized error payload extracted from a NovelAI error body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    /// The `statusCode` or `code` field when it is an int or string.
    pub code: Option<ErrorCode>,
    /// The message / error / detail text, or the raw body.
    pub message: String,
    /// The first 4096 bytes of the body, decoded lossily.
    pub raw: String,
}

/// Extract `(code, message)` from a NovelAI error body.
///
/// NovelAI returns JSON in several shapes — `{"statusCode", "message"}`,
/// `{"code", "message"}`, `{"error": ...}`, or a bare `{"message"}` — or, on
/// some legacy paths, raw text. Falls back to the decoded text when neither a
/// code nor a message field is present.
pub fn parse_error_body(content: &[u8]) -> ErrorInfo {
    let raw = String::from_utf8_lossy(&content[..content.len().min(4_096)]).into_owned();

    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return ErrorInfo {
            code: None,
            message: raw.clone(),
            raw,
        };
    };

    let code = match map.get("statusCode").or_else(|| map.get("code")) {
        Some(serde_json::Value::Number(n)) => n
            .as_u64()
            .and_then(|v| u16::try_from(v).ok().map(ErrorCode::Http)),
        Some(serde_json::Value::String(s)) => Some(ErrorCode::Api(s.clone())),
        _ => None,
    };

    let message = ["message", "error", "detail"]
        .iter()
        .filter_map(|key| map.get(*key))
        .find_map(|value| value.as_str().map(str::trim).filter(|s| !s.is_empty()));

    let message = match message {
        Some(text) => text.to_owned(),
        None => raw.clone(),
    };

    ErrorInfo { code, message, raw }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_formatting() {
        let err = Error::Authentication {
            message: "NovelAI HTTP 401: invalid token".into(),
            code: Some(ErrorCode::Http(401)),
            explanation: Some(explain(Some(&ErrorCode::Http(401))).unwrap().into()),
        };
        assert_eq!(
            err.to_string(),
            "NovelAI HTTP 401: invalid token [code 401: authentication failed \
             (missing, invalid, or expired credentials)]"
        );

        let err = Error::Provider {
            status: Some(409),
            message: "NovelAI HTTP 409: conflict".into(),
            code: Some(ErrorCode::Api("E_CONFLICT".into())),
            explanation: None,
        };
        assert_eq!(
            err.to_string(),
            "NovelAI HTTP 409: conflict [code E_CONFLICT]"
        );
    }

    #[test]
    fn parses_novelai_error_shapes() {
        let cases = [
            (
                r#"{"statusCode":429,"message":"slow down"}"#,
                Some(ErrorCode::Http(429)),
                "slow down",
            ),
            (
                r#"{"code":"E_X","message":"bad"}"#,
                Some(ErrorCode::Api("E_X".into())),
                "bad",
            ),
            (r#"{"error":"boom"}"#, None, "boom"),
            (r#"{"detail":" nope "}"#, None, "nope"),
            ("raw text", None, "raw text"),
            (r#""just a string""#, None, "\"just a string\""),
        ];
        for (body, status, message) in cases {
            let info = parse_error_body(body.as_bytes());
            assert_eq!(info.message, message, "body: {body}");
            assert_eq!(info.code, status, "body: {body}");
        }
    }

    #[test]
    fn error_body_is_capped() {
        let body = "x".repeat(10_000);
        assert_eq!(parse_error_body(body.as_bytes()).raw.len(), 4_096);
    }
}
