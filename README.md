# NovelAI Image SDK

A Rust SDK for the NovelAI image-generation API, with generated bindings for
Python, TypeScript (Node.js), Kotlin, Swift, Go, C#, and C++.

This is the **domain core** that powers
[NovelAI-Image-MCP](https://github.com/xinvxueyuan/NovelAI-Image-MCP), extracted
into a standalone library so any language can generate images, run Director
tools, transfer vibes, annotate, and read account data without reimplementing
the wire protocol.

| English | 简体中文 | 日本語 |
|---|---|---|
| [README.md](README.md) | [.github/docs/README-zh.md](.github/docs/README-zh.md) | [.github/docs/README-ja.md](.github/docs/README-ja.md) |

## Why a Rust core

- **One implementation, seven languages.** The wire protocol (payload shape,
  validation, Anlas math, error taxonomy) lives once. Bindings are generated,
  not retyped.
- **Cloudflare-safe transport.** `image.novelai.net` resets connections whose
  TLS fingerprint is not browser-like. The SDK drives
  [`wreq`](https://github.com/0x676e67/wreq) with Chrome's BoringSSL
  ClientHello, HTTP/2 settings and header block — see
  [`docs/cloudflare-spike.md`](docs/cloudflare-spike.md) for the live result.
- **Byte-level fidelity.** Golden fixtures generated from the reference Python
  implementation assert the payloads and error messages match exactly.

## Quick start (Rust)

```rust,no_run
use novelai_image_sdk::auth::Credentials;
use novelai_image_sdk::client::NovelAiClient;
use novelai_image_sdk::config::ClientConfig;
use novelai_image_sdk::models::GenerationRequest;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = NovelAiClient::new(ClientConfig::new(Credentials::token("YOUR_TOKEN")))?;

let request = GenerationRequest::new("1girl, smile")
    .width(832)
    .height(1216)
    .steps(28)
    .build()?;

for image in client.generate(&request).await? {
    std::fs::write(&image.filename, &image.data)?;
}
# Ok(())
# }
```

The `blocking` feature adds a synchronous facade backed by an owned Tokio
runtime (used by the Go and C++ bindings):

```rust,ignore
let client = BlockingNovAiClient::new(config)?;
let images = client.generate(&request)?;
```

## Feature flags

| Feature | Default | Purpose |
|---|---|---|
| `browser-emulation` | yes | Chrome TLS/HTTP-2 fingerprint via `wreq`. Disable only for offline tests — NovelAI will reject non-emulated clients. |
| `compression` | yes | Transparent gzip / brotli / zstd response decoding. |
| `blocking` | no | Synchronous facade for Go/C++ bindings and simple callers. |
| `output` | no | `save_image` / `resolve_output_path` host helpers. |

## Capabilities

| Area | API |
|---|---|
| Generation | `generate` (V3/Furry ZIP and V4+/V5 MessagePack, auto-sniffed) |
| Streaming | `stream_generation` (Rust `Stream`; callback/buffered in bindings) |
| Director tools | `director` (`lineart`, `sketch`, `bg-removal`, `declutter`, `colorize`, `emotion`) |
| Enhance | `upscale` (V5 only, 4x), `annotate` (ControlNet models, legacy host) |
| Vibes | `encode_vibe` with an LRU encode cache |
| Tags | `suggest_tags` |
| Account | `get_subscription`, `get_user_data` |
| Local | `GenerationRequest::estimate_anlas_cost`, model capability tables |

## Build prerequisites

BoringSSL is compiled from source, which needs:

- `cmake` and `nasm` on `PATH`
- `LIBCLANG_PATH` pointing at a `libclang` install (the ~20 MB PyPI `libclang`
  wheel is enough — no need for the full LLVM distribution)

Then `cargo check`, `cargo test --workspace --all-features`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`.

## Bindings (roadmap)

| Language | Status | Package |
|---|---|---|
| Python | v0.1 | `novelai-image-sdk` (PyPI) |
| TypeScript / Node.js | v0.1 | `@xinvxueyuan/novelai-image-sdk` (npm) |
| Kotlin (JVM/Android) | v0.1 | `io.github.xinvxueyuan:novelai-image-sdk` (Maven Central) |
| Swift | v0.1 | `NovelAIImageSDK` (Swift Package / XCFramework) |
| Go | v0.2 | `github.com/xinvxueyuan/NovelAI-Image-SDK/bindings/go` |
| C# /.NET | v0.2 | `NovelAI.Image.Sdk` (NuGet) |
| C++ | v0.2 | CMake package (blocking API only) |

Bindings are generated with [UniFFI](https://github.com/mozilla/uniffi-rs)
(Kotlin, Swift, Python, and — via NordSecurity generators — Go, C#, C++) and
[napi-rs](https://napi.rs) for Node.js.

## Data & privacy

Credentials are never logged. `Credentials`' `Debug`/`Display` print only the
credential *kind*. Generated images are returned as bytes; the SDK writes files
only if you call `output::save_image`.

## License

MIT. See [`LICENSE`](LICENSE), [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)
(UniFFI runtime is MPL-2.0; BoringSSL is Apache-2.0), and
[`NOTICE`](NOTICE) for upstream attribution.
