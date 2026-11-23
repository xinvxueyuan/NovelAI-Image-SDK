# Third-party notices

`novelai-image-sdk` (the Rust core crate) is MIT-licensed and has no
copyleft build dependencies. The following notices apply to dependencies and to
artifacts built by this repository.

## UniFFI (MPL-2.0)

The generated bindings and the `cdylib`/`staticlib` in
`crates/novelai-image-ffi-uniffi` link the [UniFFI](https://github.com/mozilla/uniffi-rs)
runtime, which is licensed under the **Mozilla Public License 2.0**. The full
text is at <https://www.mozilla.org/en-US/MPL/2.0/> and the source is available
at <https://github.com/mozilla/uniffi-rs>.

The MPL is file-level (weak) copyleft: it applies to UniFFI's own source files,
not to this repository's MIT-licensed code. We do not modify UniFFI. If you
distribute binaries built from this repository, include a copy of the MPL-2.0
text and a pointer to the UniFFI source, as this notice does.

The `novelai-image-sdk` crate itself does **not** depend on UniFFI; only the
`novelai-image-ffi-uniffi` boundary crate (which is not published to
crates.io) does.

## BoringSSL / btls

TLS fingerprint emulation is provided by [`wreq`](https://github.com/0x676e67/wreq)
(Apache-2.0) and its BoringSSL bindings (`btls`, MIT). BoringSSL is
[Google's fork of OpenSSL](https://boringssl.googlesource.com/boringssl/) and is
licensed under Apache-2.0
(<https://www.apache.org/licenses/LICENSE-2.0>).

Some transitive crates may be offered under MIT OR Apache-2.0; either
alternative is acceptable for this project.

## Upstream reference implementation

The wire protocol, validation rules and error taxonomy are ported from
[NovelAI-Image-MCP](https://github.com/xinvxueyuan/NovelAI-Image-MCP)
(MIT, © NovelAI Image MCP contributors). The golden fixtures under
`fixtures/golden/` are generated from that implementation.
