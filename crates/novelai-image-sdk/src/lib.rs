//! NovelAI Image SDK — Rust core.
//!
//! Ports the domain core of NovelAI-Image-MCP (`novelai_image_mcp.nai`) to Rust:
//! request validation, wire payload construction, response parsing (ZIP and
//! length-prefixed MessagePack), the Chrome-fingerprinted HTTP client, and the
//! error taxonomy.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Ported numeric protocol code; these pedantic lints fight the reference
// semantics (f64 Anlas math, u64 pixel areas, py-style naming).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::new_without_default,
    clippy::struct_excessive_bools,
    clippy::match_same_arms,
    clippy::option_if_let_else,
    clippy::items_after_statements,
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    clippy::return_self_not_must_use,
    clippy::doc_markdown,
    clippy::assigning_clones,
    clippy::unnecessary_map_or,
    clippy::implicit_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::uninlined_format_args,
    clippy::type_complexity,
    clippy::ptr_arg,
    clippy::ref_option,
    clippy::trivially_copy_pass_by_ref
)]

pub mod auth;
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod browser;
pub mod client;
pub mod config;
pub mod constants;
pub mod error;
pub mod imaging;
pub mod models;
#[cfg(feature = "output")]
pub mod output;
pub mod payload;
pub mod response;
pub mod spec;

/// SDK version, mirrored from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
