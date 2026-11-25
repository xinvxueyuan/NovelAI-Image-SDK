# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-11-25

Closes the FFI gap that kept the bindings from matching the Rust core.

### Added

- Every generation option the Rust core already supported now crosses both FFI
  boundaries: `dynamic_thresholding`, `smea`, `smea_dynamic`, `auto_smea`,
  `add_original_image`, `controlnet_strength`, `controlnet_condition`,
  `controlnet_model`, `reference_strengths`, `reference_information`,
  `character_prompts`, `use_coords`, `use_order`, `legacy_uc`,
  `normalize_reference_strengths`, `deliberate_euler_ancestral_bug`,
  `prefer_brownian`, `skip_cfg_above_sigma`, `legacy`, `legacy_v3_extend`,
  `inpaint_img2img_strength` and `straight_alpha`. Python, Kotlin, Swift,
  Node.js, Go, C#, and C++ callers were limited to a 24-field subset before.
- `NovelAiClient.collect_generation(request, max_events)` on the UniFFI
  bindings. UniFFI has no streaming export, so it drains the core's
  `stream_generation` stream into a list, matching Node's
  `collectGeneration`.

### Changed

- The UniFFI record now converts through `novelai_image_sdk::spec`, the same
  mapping the napi boundary uses, instead of a second hand-written 24-field
  conversion. A field added to the core spec is now available to both.

## [0.1.9] - 2026-11-25

Publishes the npm packages, and fixes the loader they shipped with.

### Fixed

- The npm entry point only searched its own directory for the native addon,
  which is where `napi build` puts it during local development. A published
  install never has it there — npm resolves the platform package and the addon
  lives in that directory — so `require('@xinvxueyuan/novelai-image-sdk')`
  threw "No native addon found" on every platform. It now resolves the host
  suffix, prefers a local build, falls back to the platform package, and fails
  with the list of published platforms.
- `npm` publishing goes through OIDC trusted publishing with `npm stage
  publish` instead of an access token: the version enters a review queue that a
  maintainer approves, so the account keeps 2FA and CI holds no credential. The
  job runs Node 24, because npm 10.9.9 (the Node 22 image) predates both
  trusted and staged publishing.

### Changed

- Kotlin plugin 2.1.0 → 2.4.21, JNA 5.15.0 → 5.19.1, and
  kotlinx-coroutines-core 1.10.2 → 1.11.0.
- `actions/setup-java` v4.7.1 → v6.3.0.

## [0.1.8] - 2026-11-25

### Fixed

- `cargo xtask sync-versions` matched a literal `{version}` template, so it
  never rewrote a manifest and always reported "up to date". PyPI received a
  0.1.0 wheel during the v0.1.7 release because of it. The template is now a
  regex with the version as a capture group, every match is replaced, and the
  npm platform dependencies are included.

## [0.1.7] - 2026-11-25

Publishes the PyPI wheel.

### Fixed

- The manylinux job left the unrepaird `linux_x86_64` wheel beside the
  repaired one, so the upload glob shipped both and PyPI rejected the batch.
  `auditwheel` now replaces `dist/`, and the dry run refuses to pass if a bare
  `linux_*` wheel is present.

## [0.1.6] - 2026-11-25

Publishes the PyPI wheel.

### Fixed

- The manylinux image has no interpreter on `PATH`; it ships them under
  `/opt/python`. The build step resolves the path itself, because
  `GITHUB_ENV` changes do not reach inside the container.

## [0.1.5] - 2026-11-25

Publishes the PyPI wheel.

### Fixed

- The manylinux container sets `HOME` differently from the container user's
  real home, which makes `rustup` refuse to install. `RUSTUP_HOME` and
  `CARGO_HOME` are now exported explicitly inside the container.

## [0.1.4] - 2026-11-25

Publishes the PyPI wheel.

### Fixed

- PyPI rejected the Linux wheel: the runner's glibc is too new to produce an
  installable manylinux tag. The Linux wheel now builds inside the official
  `manylinux_2_28` container and is repaired with `auditwheel`, so the tag
  reflects the symbols the binary actually needs.

## [0.1.3] - 2026-11-24

Publishes the npm packages.

### Fixed

- \`napi artifacts\` requires a separate \`artifacts/\` directory the release
  layout does not use; the addons are now routed into their platform packages
  by filename suffix, so a Windows binary cannot land in the macOS package.

## [0.1.2] - 2026-11-24

Completes the Windows and x86_64-macOS packaging.

### Fixed

- Multi-line build steps used POSIX syntax (`cp`, `for … done`, backslash
  continuations), which the Windows runner's PowerShell rejects. They are now
  single commands or a checked-in Python script.
- The npm jobs never ran `rustup target add`, so a non-host target failed with
  `can't find crate for core`.
- x86_64 macOS builds on the native Intel runner instead of cross-compiling
  BoringSSL.

## [0.1.1] - 2026-11-24

Completes the first release: the crate published as 0.1.0, and this patch
publishes the PyPI wheel, the npm addons and the macOS Swift framework.

### Fixed

- The Python wheel is retagged `py3-none-<platform>`; the extension is a
  ctypes-loaded cdylib, so it does not depend on the CPython ABI and now
  installs on every Python 3.
- The npm build on Windows forces `shell: bash`; a POSIX line continuation in
  the multi-line command fails under PowerShell.
- The Swift framework is macOS-only. BoringSSL is not cross-compiled for the
  iOS SDKs yet.

## [0.1.0] - 2026-11-24

First release: the Rust core and the Python, TypeScript, Kotlin and Swift
bindings, described in [`release-notes/0.1.0.json`](release-notes/0.1.0.json).

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
  identical constants, Argon2 access keys, identical Anlas estimates and
  identical error messages.
- Live transport spike (`cargo xtask live-spike`) confirming the Chrome
  emulation reaches NovelAI's origin through Cloudflare.
- Bindings: Python (`novelai-image-sdk` on PyPI), TypeScript/Node
  (`@xinvxueyuan/novelai-image-sdk`), Kotlin/JVM
  (`io.github.xinvxueyuan:novelai-image-sdk`) and Swift (`NovelAIImageSDK`).
- Repository gate: `cargo xtask check` runs fmt, clippy, nextest, cargo-deny,
  REUSE, typos, tombi, the file-length cap and prek hooks, and CI runs the same
  set on the newest Linux, Windows and macOS runners.

### Known limitations

- Streaming is buffered in the bindings; only the Rust core exposes the
  incremental `Stream`.
- Go, C# and C++ bindings are planned for v0.2.
- The browser/WASM target is out of scope: the transport depends on controlling
  the TLS ClientHello.

## [0.0.0] - 2026-11-23

- Repository bootstrap.
