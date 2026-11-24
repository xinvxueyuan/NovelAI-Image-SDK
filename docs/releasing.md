# Releasing

A release is triggered by pushing a tag. The `release.yml` workflow refuses a
tag whose version disagrees with `Cargo.toml`, whose commit is not on `main`, or
that lacks release notes, and it runs the full CI gate before publishing
anything.

## One-time setup

These are the credentials the workflow needs. Nothing is published until all of
them exist, so set them up before the first tag.

| Store | Secret / setting | Where it comes from |
|---|---|---|
| crates.io | `CARGO_REGISTRY_TOKEN` (repository secret) | <https://crates.io/settings/tokens>, scoped to `publish-new` + `publish-update` for this crate |
| PyPI | Trusted Publishing (no secret) | Add a trusted publisher on PyPI: owner `xinvxueyuan`, repository `NovelAI-Image-SDK`, workflow `release.yml`, environment `release` |
| npm | `NPM_TOKEN` (repository secret) | An npm automation token for the `@xinvxueyuan` scope |
| Maven Central | `MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` | Central Portal token; the Gradle publish task reads these |
| GitHub | `GITHUB_TOKEN` | Automatic; the release job needs `contents: write` |

The `crate` job uses the `release` environment, so a required reviewer can gate
publication there.

## Steps

1. **Pick the version.** `Cargo.toml`'s `[workspace.package].version` is the
   only hand-edited version.

2. **Update the changelog.** Rename `## [Unreleased]` to
   `## [X.Y.Z] - YYYY-MM-DD` and leave a fresh `## [Unreleased]` section.

3. **Write `release-notes/X.Y.Z.json`.** It is the GitHub release body; the
   `validate` job fails without it. See `release-notes/0.1.0.json` for the shape.

4. **Sync the manifests.**

   ```bash
   cargo xtask sync-versions          # writes the Python/npm/Kotlin/Swift manifests
   cargo xtask sync-versions --check  # what the release workflow verifies
   ```

5. **Regenerate the bindings** if the FFI surface changed:

   ```bash
   cargo xtask bindings
   ```

6. **Run the gate.**

   ```bash
   cargo xtask check
   ```

7. **Commit and tag on `main`.**

   ```bash
   git commit -s -m "🏷️ chore(release): X.Y.Z"
   git push origin main
   git tag -s vX.Y.Z -m "vX.Y.Z"
   git push origin vX.Y.Z
   ```

8. **Watch it.**

   ```bash
   gh run watch
   ```

## What gets published

| Artifact | Job | Destination |
|---|---|---|
| `novelai-image-sdk` crate | `crate` | crates.io |
| `novelai_image_sdk-*.whl` | `python-wheels` | GitHub release (PyPI upload is a follow-up step) |
| `*.node` addons | `npm` | GitHub release (npm publish is a follow-up step) |
| `NovelAIImageSDK.xcframework.zip` | `swift` | GitHub release; the checksum goes into `bindings/swift/Package.swift` |
| Release notes | `github-release` | GitHub release |

### Known gaps before the first tag

- **PyPI upload** is not wired yet: the wheels are built and attached to the
  release, but `twine upload` (or PyPI Trusted Publishing) is a separate step.
  Add it to `python-wheels` once the trusted publisher exists.
- **npm publish** is not wired yet for the same reason: the addons are attached
  to the release; `npm publish` per platform package is a follow-up.
- **Maven Central** publishing needs the Gradle `publish` task run with a
  bundled native library per platform; the current job builds the JAR but does
  not attach the `.dll`/`.so`/`.dylib` resources. Do that before publishing.
- **Swift**: `Package.swift` carries a placeholder checksum until the first
  XCFramework is published; replace it with the value the `swift` job prints.
- **PyPI re-upload**: a version is immutable. Fix a bad release with a patch
  version, never by re-uploading the same one.
