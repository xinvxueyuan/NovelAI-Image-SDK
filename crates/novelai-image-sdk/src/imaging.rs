//! PNG/JPEG validation and dimension sniffing without an imaging dependency.
//!
//! Port of `novelai_image_mcp.nai.imaging`.

use base64::Engine as _;

use crate::error::Error;

/// Validated image bytes plus their dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedImage {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// The original, unmodified bytes.
    pub data: Vec<u8>,
}

impl ParsedImage {
    /// The image bytes as standard base64 (what the API expects inline).
    pub fn base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }
}

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const JPEG_SOI: &[u8; 3] = b"\xff\xd8\xff";

fn png_dimensions(data: &[u8]) -> Result<(u32, u32), Error> {
    if data.len() < 24 {
        return Err(Error::image("truncated PNG image"));
    }
    let width = u32::from_be_bytes(data[16..20].try_into().expect("4 bytes"));
    let height = u32::from_be_bytes(data[20..24].try_into().expect("4 bytes"));
    Ok((width, height))
}

fn jpeg_dimensions(data: &[u8]) -> Result<(u32, u32), Error> {
    let mut pos = 2usize;
    while pos + 4 <= data.len() {
        let marker = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        if marker == 0xFFD8 || marker == 0xFFD9 {
            continue;
        }
        if pos + 2 > data.len() {
            break;
        }
        let segment_length = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        if segment_length < 2 {
            break;
        }
        if (0xFFC0..0xFFC4).contains(&marker)
            || (0xFFC5..0xFFC8).contains(&marker)
            || (0xFFC9..0xFFCC).contains(&marker)
        {
            if pos + 5 > data.len() {
                break;
            }
            let payload = &data[pos..pos + 5];
            let width = u16::from_be_bytes([payload[3], payload[4]]) as u32;
            let height = u16::from_be_bytes([payload[1], payload[2]]) as u32;
            return Ok((width, height));
        }
        pos += segment_length - 2;
    }
    Err(Error::image("JPEG dimensions were not found"))
}

/// Validate PNG/JPEG bytes and return dimensions plus the original data.
pub fn parse_image(data: &[u8]) -> Result<ParsedImage, Error> {
    let (width, height) = if data.starts_with(PNG_SIGNATURE) {
        png_dimensions(data)?
    } else if data.starts_with(JPEG_SOI) {
        jpeg_dimensions(data)?
    } else {
        return Err(Error::image("only PNG and JPEG images are supported"));
    };
    if width == 0 || height == 0 {
        return Err(Error::image("image dimensions must be positive"));
    }
    Ok(ParsedImage {
        width,
        height,
        data: data.to_vec(),
    })
}

/// A 1x1 transparent PNG fixture (shared by unit, integration and
/// binding-level smoke tests).
pub const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_png_dimensions() {
        let parsed = parse_image(TINY_PNG).unwrap();
        assert_eq!((parsed.width, parsed.height), (1, 1));
        assert!(parsed.base64().starts_with("iVBORw0KGgo"));
    }

    #[test]
    fn rejects_unknown_and_truncated() {
        assert!(matches!(
            parse_image(b"GIF89a").unwrap_err(),
            Error::Image { .. }
        ));
        assert!(matches!(
            parse_image(&TINY_PNG[..10]).unwrap_err(),
            Error::Image { .. }
        ));
    }
}
