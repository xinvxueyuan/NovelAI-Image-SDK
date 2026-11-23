# AGENTS.md

Agent-facing context index. Developer docs live in
[`CONTRIBUTING.md`](CONTRIBUTING.md); the wire protocol notes are in
[`docs/cloudflare-spike.md`](docs/cloudflare-spike.md) and
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

## What this is

A Rust SDK (`novelai-image-sdk`) for NovelAI's image-generation API, ported
from the domain core of
[NovelAI-Image-MCP](https://github.com/xinvxueyuan/NovelAI-Image-MCP), with
generated bindings for Python, TypeScript/Node, Kotlin, Swift (v0.1) and Go,
C#, C++ (v0.2).

## Layout

```text
crates/novelai-image-sdk/       # the published core crate (MIT, no uniffi)
  src/auth.rs                   # Argon2id access key, tracking headers
  src/browser.rs                # the ONE Chrome fingerprint definition
  src/client.rs                 # NovelAiClient (all endpoints)
  src/config.rs                 # ClientConfig
  src/constants.rs              # models/samplers/actions/presets/capabilities
  src/error.rs                  # Error enum + NovelAI code explanations
  src/imaging.rs                # PNG/JPEG sniffing (no image deps)
  src/models.rs                 # GenerationRequest builder + Anlas math
  src/payload.rs                # V3 / V4+ / V5 wire payloads
  src/response.rs               # status mapping, ZIP, MessagePack framing
  src/blocking.rs               # feature `blocking`: sync facade (Go/C++)
crates/novelai-image-ffi-uniffi/  # UniFFI boundary (publish = false)
crates/novelai-image-ffi-napi/    # napi-rs boundary (publish = false)
tools/xtask/                    # live-spike, fixtures, bindings, version sync
fixtures/golden/                # generated from the Python reference; committed
scripts/gen_fixtures.py         # regenerates them
```

## Hard rules (do not break)

1. **Never use plain `reqwest`/rustls/native-tls.** `image.novelai.net` sits
   behind Cloudflare and resets non-browser TLS fingerprints. All traffic goes
   through `BrowserProfile` → `wreq` (BoringSSL). `cargo deny` bans those
   crates. See `docs/cloudflare-spike.md` for the proven configuration.
2. **Keep the emulation header injection off.** `wreq`'s `Emulation` adds its
   own header block; we call `.headers(false)` and write ours once in
   `base_request_with`. Enabling it duplicates `User-Agent`/`Sec-Ch-Ua`/
   `Accept` and breaks streaming negotiation.
3. **Exactly one `Accept` per request** (`*/*`, or
   `application/x-msgpack` for streaming). `wreq` keeps duplicate singleton
   headers joined by commas.
4. **Do not hand-edit `fixtures/golden/`.** Regenerate with
   `scripts/gen_fixtures.py`; CI fails if the tree is dirty after
   regeneration.
5. **Byte fidelity is the contract.** Payload key order, float formatting and
   error message text (including the ` [code X: explanation]` suffix) are
   asserted against the reference. Do not "clean up" messages.
6. **`GenerationRequest` is validated via the builder.** `GenerationRequest::new(..).build()`
   replays the Python `__post_init__` order. `extra_noise_seed` randomness is
   injectable (`build_with_rng`) so fixtures can be replayed.
7. **`estimate_only` relaxes exactly two checks** (image requirement, inpaint
   mask/model); price inputs are still validated.
8. **Blocking facade must not be called from inside an async runtime**; it
   returns `Error::Config` rather than panicking.
9. **MPL-2.0 isolation:** only `crates/novelai-image-ffi-uniffi` may depend on
   `uniffi`. The core crate must stay free of it.
10. **Actions are SHA-pinned**; commits are gitmoji + Conventional Commits
    with DCO sign-off (`--no-verify` is forbidden).
11. **The clippy policy is stricter than default** (`clippy.toml`): functions
    over 60 lines and blocks nested deeper than 3 are errors. Split by role
    (production into modules, tests beside them) — every file must stay at or
    below 500 lines (`scripts/check-file-length.mjs`).
12. **`rust-version`, the `rust-toolchain.toml` channel and `clippy.toml` msrv
    move together.** Change all three in the same commit.

## Common commands

```bash
cargo xtask check                           # the whole gate: fmt, clippy, nextest,
                                           # deny, REUSE, typos, tombi, file length, prek
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
uvx --from "reuse[charset-normalizer]" reuse lint
cargo run -p xtask -- live-spike            # requires a valid NOVELAI_TOKEN for 200
NOVELAI_REFERENCE_ROOT=../NovelAI-Image-MCP \
  uv run --directory ../NovelAI-Image-MCP/apps/server \
  python "$PWD/scripts/gen_fixtures.py"
```

## Build prerequisites

`cmake`, `nasm`, and `LIBCLANG_PATH` (a `libclang` install) — BoringSSL is
compiled from source. On Windows the PyPI `libclang` wheel is sufficient.

## Don't

- ❌ Add a plain HTTP client or "simplify" the transport to reqwest.
- ❌ Enable `wreq`'s emulation header block.
- ❌ Change error message text, payload key order, or defaults without
  regenerating fixtures and reviewing the diff.
- ❌ Publish `crates/novelai-image-ffi-*` to crates.io.
- ❌ Put uniffi in the core crate.
- ❌ Add a GitHub Action with a floating tag (`@v4`); pin the full SHA.
