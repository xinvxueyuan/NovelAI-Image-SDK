//! napi-rs boundary crate. Populated in phase P3.

#![allow(missing_docs)]

use napi_derive::napi;

/// SDK version.
#[napi]
pub fn version() -> &'static str {
    novelai_image_sdk::VERSION
}
