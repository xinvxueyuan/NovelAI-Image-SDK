# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `novelai-image-sdk` Rust core: request validation and normalization
  (`GenerationRequest` builder), V3 / V4+ / V5 wire payloads, ZIP and
  length-prefixed MessagePack parsing, Anlas cost estimation, the error
  taxonomy with NovelAI error codes/explanations, and the model capability
  tables.
- Chrome-fingerprinted HTTP client (`wreq` + BoringSSL) for every endpoint:
  generation, streaming, Director tools, upscale, annotate, vibe encoding, tag
  suggestions and account reads. Login with username/password (Argon2id key
  derivation) is cached at most once and retryable after failure.
- `blocking` feature with a synchronous facade for Go and C++ bindings.
- Golden-fixture parity tests generated from the reference Python
  implementation: byte-identical payloads across 21 model/action/preset cases,
  identical constants, Argon2 access keys, and identical error messages.
- Live transport spike (`cargo xtask live-spike`) confirming the Chrome
  emulation reaches NovelAI's origin through Cloudflare.

## [0.0.0] - 2026-11-23

- Repository bootstrap.
