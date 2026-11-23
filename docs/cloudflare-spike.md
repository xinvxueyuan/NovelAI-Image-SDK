# Phase 0 spike: does wreq's Chrome profile clear NovelAI's Cloudflare WAF?

`image.novelai.net` fingerprints clients at two layers (see the source repo's
`nai/http.py` docstring):

1. **TLS (JA3/JA4) + HTTP/2 fingerprint** — plain OpenSSL/rustls handshakes are
   reset before any header is read.
2. **Browser headers** — User-Agent, `Sec-Ch-Ua*`, `Sec-Fetch-*`, `Origin`,
   `Referer`.

Python solved this with `curl_cffi` (curl-impersonate, BoringSSL). In Rust the
candidate is [`wreq`](https://github.com/0x676e67/wreq) 0.16 with
`wreq-util::Emulation::Chrome150`, which reproduces Chrome's BoringSSL
ClientHello, HTTP/2 SETTINGS and pseudo-header order.

## Running the spike

```powershell
# Unauthenticated: a JSON 401 means the request reached NovelAI (not Cloudflare).
cargo run -p xtask -- live-spike

# Authenticated (requires a real token in $env:NOVELAI_TOKEN):
cargo run -p xtask -- live-spike --path /ai/generate-image/suggest-tags
```

A `403` page whose body mentions Cloudflare, a TLS reset (`transport: ...`), or
a timeout all mean the fingerprint was rejected.

## Results

| Date | wreq | Profile | Build env | Unauthenticated `/user/subscription` | Notes |
|---|---|---|---|---|---|
| 2026-11-23 | 0.16.4 | `Emulation::Chrome149` | Windows 10 x64, MSVC, BoringSSL built from source (cmake 4.5.1 + NASM 3.02 + libclang from the PyPI `libclang` wheel) | **HTTP 401 `{"statusCode":401,"message":"Unauthorized"}`** | **PASS** — a JSON 401 from NovelAI's origin proves the TLS/JA3/JA4 + HTTP/2 fingerprint cleared Cloudflare. |

### Decision

`wreq` + `wreq-util::Emulation` is adopted as the transport. `wreq-util` 0.2.1
(the newest published release) ships profiles up to **Chrome149**; the reference
Python client pins Chrome **150** headers. One version behind is still a genuine
Chrome fingerprint and the request reached the origin, so the plan's
`Chrome150` target becomes `Chrome149` until `wreq-util` publishes a newer
profile. The version lives in a single place (`browser.rs`'s `BrowserProfile`)
so the UA / `Sec-Ch-Ua` / emulation stay in lock-step.

### Build prerequisites (Windows)

- `cmake`, `nasm` (BoringSSL compiles ~370 translation units; cached by `btls-sys`).
- `LIBCLANG_PATH` for `bindgen` — the PyPI `libclang` wheel
  (`pip install libclang`) is a ~20 MB alternative to the 610 MB LLVM MSI.
- Re-run with `cargo run -p xtask -- live-spike` after `cargo clean`.
