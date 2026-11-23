//! NovelAI HTTP, ZIP and MessagePack response parsing.
//!
//! Port of `novelai_image_mcp.nai.response`.

use std::io::{Cursor, Read};

use serde::Deserialize;

use crate::error::{Error, ErrorCode, explain, parse_error_body};

/// An image returned by NovelAI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NovelAIImage {
    /// Suggested filename (from a ZIP entry or synthesized from the event).
    pub filename: String,
    /// Raw image bytes (PNG or JPEG).
    pub data: Vec<u8>,
}

/// A MessagePack streaming-generation event.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationEvent {
    /// `"intermediate"` or `"final"`.
    pub event_type: String,
    /// Index of the sample (`samp_ix`).
    pub sample_index: i64,
    /// Sampling step (`step_ix`).
    pub step_index: i64,
    /// Generation id (`gen_id`).
    pub generation_id: String,
    /// Noise sigma, when reported.
    pub sigma: f64,
    /// The image at this step.
    pub image: NovelAIImage,
}

/// Maximum MessagePack frame size (128 MiB), matching the Python parser.
pub const MAX_FRAME_BYTES: u32 = 128 * 1024 * 1024;

/// Raise the domain error associated with an HTTP status.
///
/// The NovelAI error code (body `statusCode`/`code`, or the HTTP status when
/// the body carries none) and its official explanation are attached to the
/// error and appended to its message.
pub fn check_status(status: u16, content: &[u8]) -> Result<(), Error> {
    if status < 400 {
        return Ok(());
    }
    let info = parse_error_body(content);
    let code = info.code.clone().unwrap_or(ErrorCode::Http(status));
    let explanation = explain(Some(&code)).or_else(|| explain(Some(&ErrorCode::Http(status))));
    let message = format!("NovelAI HTTP {status}: {}", info.message);
    let explanation = explanation.map(str::to_owned);

    Err(match status {
        400 => Error::Validation {
            message,
            code: Some(code),
            explanation,
        },
        401 => Error::Authentication {
            message,
            code: Some(code),
            explanation,
        },
        402 => Error::InsufficientCredits {
            message,
            code: Some(code),
            explanation,
        },
        429 => Error::Concurrency {
            message,
            code: Some(code),
            explanation,
        },
        _ => Error::Provider {
            status: Some(status),
            message,
            code: Some(code),
            explanation,
        },
    })
}

/// Parse every file in a ZIP response, skipping directory entries.
pub fn parse_zip_images(content: &[u8]) -> Result<Vec<NovelAIImage>, Error> {
    let invalid = || Error::response("NovelAI returned an invalid ZIP response");
    let mut archive = zip::ZipArchive::new(Cursor::new(content)).map_err(|_| invalid())?;
    let mut images = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| invalid())?;
        let filename = entry.name().map_err(|_| invalid())?.into_owned();
        if filename.ends_with('/') {
            continue;
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data).map_err(|_| invalid())?;
        images.push(NovelAIImage { filename, data });
    }
    Ok(images)
}

/// True when the body is a ZIP archive (`PK` signature).
pub fn is_zip(content: &[u8]) -> bool {
    content.starts_with(b"PK")
}

/// Decode one MessagePack frame into an event, if it carries one.
fn decode_frame(payload: &[u8]) -> Result<Option<GenerationEvent>, Error> {
    let mut deserializer = rmp_serde::Deserializer::new(Cursor::new(payload));
    let value = rmpv::Value::deserialize(&mut deserializer)
        .map_err(|_| Error::response("invalid MessagePack frame"))?;
    // Reject trailing bytes after a complete value (`ExtraData` parity).
    if deserializer.position() as usize != payload.len() {
        return Err(Error::response("invalid MessagePack frame"));
    }
    event(value)
}

fn event(value: rmpv::Value) -> Result<Option<GenerationEvent>, Error> {
    let rmpv::Value::Map(entries) = value else {
        return Ok(None);
    };
    let lookup = |key: &str| {
        entries
            .iter()
            .find(|(k, _)| k.as_str() == Some(key))
            .map(|(_, v)| v)
    };
    let event_type = lookup("event_type").and_then(rmpv::Value::as_str);
    match event_type {
        Some("retry") => return Ok(None),
        Some("error") => {
            let code = lookup("code").and_then(|c| match c {
                rmpv::Value::Integer(i) => i
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok().map(ErrorCode::Http)),
                rmpv::Value::String(s) => s.as_str().map(|s| ErrorCode::Api(s.to_owned())),
                _ => None,
            });
            let message = lookup("message")
                .and_then(rmpv::Value::as_str)
                .unwrap_or("unknown error");
            let explanation = explain(code.as_ref()).map(str::to_owned);
            return Err(Error::Provider {
                status: None,
                message: format!("NovelAI stream error: {message}"),
                code,
                explanation,
            });
        }
        Some("intermediate" | "final") => {}
        _ => return Ok(None),
    }
    let Some(rmpv::Value::Binary(image)) = lookup("image") else {
        return Ok(None);
    };
    let event_type = event_type.expect("checked above").to_owned();
    let sample_index = lookup("samp_ix").and_then(rmpv::Value::as_i64).unwrap_or(0);
    let extension = if image.starts_with(b"\x89PNG") {
        "png"
    } else {
        "jpg"
    };
    Ok(Some(GenerationEvent {
        event_type: event_type.clone(),
        sample_index,
        step_index: lookup("step_ix").and_then(rmpv::Value::as_i64).unwrap_or(0),
        generation_id: lookup("gen_id")
            .and_then(rmpv::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        sigma: lookup("sigma").and_then(rmpv::Value::as_f64).unwrap_or(0.0),
        image: NovelAIImage {
            filename: format!("sample-{sample_index}-{event_type}.{extension}"),
            data: image.to_vec(),
        },
    }))
}

/// Incrementally parse NovelAI's big-endian length-prefixed MessagePack.
///
/// Each frame is `[4-byte BE u32 length][msgpack payload]`. Chunks may split
/// frames arbitrarily.
#[derive(Debug, Default)]
pub struct MessagePackStreamParser {
    buffer: Vec<u8>,
    expected: Option<u32>,
}

impl MessagePackStreamParser {
    /// A fresh parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk; returns every event completed by it.
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<GenerationEvent>, Error> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        while let Some(payload) = self.next_frame()? {
            events.extend(decode_frame(&payload)?);
        }
        Ok(events)
    }

    /// Take the next complete frame from the buffer, if one is available.
    ///
    /// Reads the big-endian length prefix on demand and enforces
    /// [`MAX_FRAME_BYTES`].
    fn next_frame(&mut self) -> Result<Option<Vec<u8>>, Error> {
        let Some(expected) = self.frame_length()? else {
            return Ok(None);
        };
        if self.buffer.len() < expected {
            return Ok(None);
        }
        self.expected = None;
        Ok(Some(self.buffer.drain(..expected).collect()))
    }

    /// The byte length of the frame currently being assembled.
    fn frame_length(&mut self) -> Result<Option<usize>, Error> {
        if let Some(expected) = self.expected {
            return Ok(Some(expected as usize));
        }
        let Some(declared) = self.read_declared_length()? else {
            return Ok(None);
        };
        self.expected = Some(declared);
        Ok(Some(declared as usize))
    }

    /// Consume the 4-byte length prefix, validating the frame cap.
    fn read_declared_length(&mut self) -> Result<Option<u32>, Error> {
        if self.buffer.len() < 4 {
            return Ok(None);
        }
        let frame_length = u32::from_be_bytes(self.buffer[..4].try_into().expect("4 bytes"));
        self.buffer.drain(..4);
        if frame_length > MAX_FRAME_BYTES {
            return Err(Error::response("MessagePack frame is too large"));
        }
        Ok(Some(frame_length))
    }

    /// Verify the stream ended on a frame boundary.
    pub fn finish(self) -> Result<(), Error> {
        if self.expected.is_some() || !self.buffer.is_empty() {
            return Err(Error::response("truncated MessagePack stream"));
        }
        Ok(())
    }
}

/// Parse every event in a complete MessagePack body.
pub fn parse_messagepack_events(content: &[u8]) -> Result<Vec<GenerationEvent>, Error> {
    let mut parser = MessagePackStreamParser::new();
    let events = parser.feed(content)?;
    parser.finish()?;
    Ok(events)
}

/// Parse the `final` images in a complete MessagePack body.
pub fn parse_messagepack_images(content: &[u8]) -> Result<Vec<NovelAIImage>, Error> {
    Ok(parse_messagepack_events(content)?
        .into_iter()
        .filter(|event| event.event_type == "final")
        .map(|event| event.image)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut out = (bytes.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(bytes);
        out
    }

    #[test]
    fn build_zip_roundtrip() {
        let png = crate::imaging::TINY_PNG;
        let mut buffer = Vec::new();
        {
            use std::io::Write;
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buffer));
            writer
                .start_file::<_, ()>("dir/", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer
                .start_file("image_0.png", zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(png).unwrap();
            writer.finish().unwrap();
        }
        let images = parse_zip_images(&buffer).unwrap();
        assert_eq!(images.len(), 1, "directory entries must be skipped");
        assert_eq!(images[0].filename, "image_0.png");
        assert_eq!(images[0].data, png);
        assert!(matches!(
            parse_zip_images(b"not a zip").unwrap_err(),
            Error::Response { .. }
        ));
    }

    fn sample_map() -> rmpv::Value {
        rmpv::Value::Map(vec![
            ("event_type".into(), "final".into()),
            ("samp_ix".into(), 0.into()),
            ("step_ix".into(), 27.into()),
            ("gen_id".into(), "abc".into()),
            ("sigma".into(), 0.5.into()),
            (
                "image".into(),
                rmpv::Value::Binary(crate::imaging::TINY_PNG.to_vec()),
            ),
        ])
    }

    fn pack(value: &rmpv::Value) -> Vec<u8> {
        let mut out = Vec::new();
        rmpv::encode::write_value(&mut out, value).unwrap();
        out
    }

    #[test]
    fn parses_framed_events() {
        let body = frame(&pack(&sample_map()));
        let events = parse_messagepack_events(&body).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "final");
        assert_eq!(events[0].sample_index, 0);
        assert_eq!(events[0].image.filename, "sample-0-final.png");
        assert_eq!(parse_messagepack_images(&body).unwrap().len(), 1);
    }

    #[test]
    fn chunk_splitting_and_truncation() {
        let body = frame(&pack(&sample_map()));
        let mut parser = MessagePackStreamParser::new();
        let mut events = Vec::new();
        for chunk in body.chunks(3) {
            events.extend(parser.feed(chunk).unwrap());
        }
        parser.finish().unwrap();
        assert_eq!(events.len(), 1);

        let mut parser = MessagePackStreamParser::new();
        assert_eq!(parser.feed(&body[..body.len() - 1]).unwrap().len(), 0);
        assert!(matches!(
            parser.finish().unwrap_err(),
            Error::Response { .. }
        ));
    }

    #[test]
    fn retry_ignored_error_raised() {
        let retry = rmpv::Value::Map(vec![("event_type".into(), "retry".into())]);
        assert_eq!(
            parse_messagepack_events(&frame(&pack(&retry)))
                .unwrap()
                .len(),
            0
        );

        let error = rmpv::Value::Map(vec![
            ("event_type".into(), "error".into()),
            ("code".into(), "E_QUOTA".into()),
            ("message".into(), "out of anlas".into()),
        ]);
        let err = parse_messagepack_events(&frame(&pack(&error))).unwrap_err();
        assert_eq!(
            err.to_string(),
            "NovelAI stream error: out of anlas [code E_QUOTA]"
        );
    }

    #[test]
    fn oversized_frame_rejected() {
        let header = (MAX_FRAME_BYTES + 1).to_be_bytes();
        let mut parser = MessagePackStreamParser::new();
        assert!(matches!(
            parser.feed(&header).unwrap_err(),
            Error::Response { .. }
        ));
    }

    #[test]
    fn status_mapping() {
        assert!(check_status(200, b"").is_ok());
        let err = check_status(402, br#"{"statusCode":402,"message":"subscribe"}"#).unwrap_err();
        assert!(matches!(err, Error::InsufficientCredits { .. }));
        assert_eq!(err.code(), Some(&ErrorCode::Http(402)));
        assert!(
            err.to_string()
                .contains("[code 402: active subscription or Anlas required]")
        );

        let err = check_status(500, b"oops").unwrap_err();
        assert!(matches!(err, Error::Provider { .. }));
    }
}
