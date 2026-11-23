# Repository infrastructure

The toolchain and repository hygiene below are adopted from
[Axolotl-Superlogical](C:\dev\external\Axolotl-Superlogical) (Apache/GPL-3.0
project, infrastructure patterns only — no code is copied). The goal is that a
contributor moving between the two repositories sees the same gates and the
same file layout.

| Area | Adopted from Axolotl | Notes |
|---|---|---|
| Rust pin | `rust-toolchain.toml` with `channel`, `profile = "minimal"`, explicit components | Kept equal to `rust-version` in `Cargo.toml` and `msrv` in `clippy.toml`; change all three together. |
| Clippy policy | `clippy.toml` with `msrv`, `too-many-lines-threshold = 60`, `excessive-nesting-threshold = 3` | The nesting gate is why `client.rs`, `models.rs` and both FFI crates were split by role. |
| Dependency policy | `deny.toml` with `all-features`, `yanked = "deny"`, licence allow-list, wildcard bans | Bans `reqwest` / `native-tls` / `hyper-rustls`: the transport must stay `wreq` + BoringSSL. |
| Unused dependencies | `cargo shear` | Caught two unused deps at adoption. |
| Test runner | `cargo-nextest` with `.config/nextest.toml` (`ci` profile retries) | Local and CI read the same profile file. |
| Local hooks | `prek.toml` (pre-commit / pre-push / commit-msg) | Whitespace, encoding, structured-file parsing, large files, secrets, DCO trailer. The repo's own `commit-msg` hook (`.githooks/`) additionally enforces gitmoji + Conventional Commits. |
| Spellcheck | `_typos.toml` | Domain vocabulary (`anlas`, `smea`, `hed`, …) declared once. |
| TOML lint/format | `tombi.toml` | `Cargo.toml` pinned to the schemastore schema. |
| Code file length | `scripts/check-file-length.mjs` (500 physical lines) | Generated bindings are excluded. |
| CI shape | `ci.yml` → `repo-check.yml` (guardrails / dependencies / hooks) + per-OS Rust matrix + `ci-result` | Composite `.github/actions/setup-rust` installs BoringSSL prerequisites and caches. |
| Action pinning | Full commit SHA + tag comment | `dependabot.yml` groups cargo / npm / github-actions updates. |
| PR template | Summary / Motivation / Changes / Testing / Breaking Changes / Related | |

## Differences from Axolotl

- **No pnpm workspace.** Axolotl is a Tauri/GPUI desktop app; this repository's
  JavaScript is limited to `scripts/check-file-length.mjs`, run by Node
  directly (`node scripts/check-file-length.mjs`) instead of a Turbo pipeline.
  There is no `package.json`-driven toolchain to adopt.
- **BoringSSL prerequisites.** Axolotl installs GPUI's system libraries; this
  repository installs `cmake`/`nasm`/`libclang` because `btls-sys` compiles
  BoringSSL from source.
- **Golden-fixture gate.** No Axolotl equivalent; the parity job regenerates
  fixtures from the pinned Python reference and fails on drift.
- **English-only commit messages.** Axolotl's `scripts/check-commit-message.ts`
  is not adopted: this project's commits are gitmoji + Conventional Commits
  with a DCO trailer, and the existing `.githooks/commit-msg` already enforces
  that shape.

## Running the gate

```bash
cargo xtask check
```

Runs, in order: `cargo fmt --check`, `cargo clippy -D warnings`,
`cargo nextest run`, `cargo deny check`, `reuse lint`, `typos`, `tombi lint`,
`node scripts/check-file-length.mjs`, and `prek run --all-files --stage
pre-commit`. CI runs the same set plus `cargo shear` and the golden job.
