# Contributing

## Prerequisites

BoringSSL is built from source by `btls-sys`:

- **cmake** and **nasm** on `PATH`
- **libclang** for `bindgen`: set `LIBCLANG_PATH` to a directory containing
  `libclang.dll`/`libclang.so`. The PyPI `libclang` wheel is ~20 MB and is
  enough; installing the full LLVM distribution (610 MB) is unnecessary.

## Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
uvx --from "reuse[charset-normalizer]" reuse lint
```

CI runs the same set on Linux, Windows and macOS.

## Golden fixtures

`fixtures/golden/*.json` is generated from a pinned checkout of
[NovelAI-Image-MCP](https://github.com/xinvxueyuan/NovelAI-Image-MCP) and
**committed**. After any change to validation, payload construction, constants
or error formatting:

```bash
NOVELAI_REFERENCE_ROOT=../NovelAI-Image-MCP \
  uv run --directory ../NovelAI-Image-MCP/apps/server \
  python "$PWD/scripts/gen_fixtures.py"
cargo test -p novelai-image-sdk --test golden
git diff fixtures/   # review every change — this is the behavior diff
```

The CI `golden` job regenerates the fixtures and fails if the working tree is
dirty, so an un-regenerated fixture is a red build.

Never hand-edit a fixture.

## Transport invariants

`browser.rs` is the single source of truth for the Chrome fingerprint. Every
request goes through `NovelAiClient::base_request_with`, which:

1. applies `wreq`'s emulation profile **with header injection disabled**
   (`Emulation::builder().headers(false)`), because `wreq`'s own header block
   conflicts with ours (Chrome149 macOS UA vs Chrome150 Windows UA) and
   duplicates every singleton header;
2. writes the single Chrome 150 header block from `BrowserProfile::headers`;
3. sets exactly one `Accept` per request (`*/*`, or
   `application/x-msgpack` when streaming) — never rely on header
   concatenation;
4. appends fresh `x-correlation-id` / `x-initiated-at` and the bearer token.

`tests/client_wiremock.rs` asserts the header block and every request body
against a mock server. If you touch the transport, those tests are the
contract. The live spike (`cargo xtask live-spike`) must also still reach the
origin.

## Commits and PRs

- **gitmoji + Conventional Commits**, e.g.
  `🐛 fix(client): send Accept once per request`.
- A `commit-msg` hook appends `Signed-off-by` (DCO). Do not use
  `--no-verify`.
- Third-party GitHub Actions must be pinned to a full commit SHA, with the
  original tag in a comment.
- Never commit credentials, tokens, or generated images.
