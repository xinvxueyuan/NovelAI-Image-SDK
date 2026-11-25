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
| npm | Trusted Publishing (no secret) | See [npm trusted publishing](#npm-trusted-publishing) below |
| Maven Central | `MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` | Central Portal token; the Gradle publish task reads these |
| GitHub | `GITHUB_TOKEN` | Automatic; the release job needs `contents: write` |

The `crate` job uses the `release` environment, so a required reviewer can gate
publication there.

### npm trusted publishing

The npm job holds **no token**. It authenticates with an OIDC ID token, which
GitHub mints for the job and which never leaves the run. Two consequences are
worth knowing before touching it:

- **The job must run Node 24.** Trusted publishing needs npm ≥ 11.5.1 and staged
  publishing needs ≥ 11.15.0; the Node 22 image ships npm 10.9.9, which silently
  falls back to token auth and then fails with `ENEEDAUTH`.
- **A version lands in the review queue, not on the registry.** CI runs
  `npm stage publish`, and a maintainer approves it with a one-time password at
  <https://www.npmjs.com/package/@xinvxueyuan/novelai-image-sdk> → Staged
  Packages. That is the point: the account keeps 2FA enforced and CI holds no
  long-lived credential.

Register a trusted publisher for **each of the six packages** (the root package
and the five platform packages), all pointing at the same place:

| Field | Value |
|---|---|
| Provider | GitHub Actions |
| Organization / user | `xinvxueyuan` |
| Repository | `NovelAI-Image-SDK` |
| Workflow filename | `release.yml` |
| Environment | `release` |

The environment must match the job: `npm-publish` declares `environment:
release`, and OIDC refuses a mismatch.

npm exposes this as `npm trust github` (interactive, 2FA, npm ≥ 11.15.0):

```bash
npm trust github @xinvxueyuan/novelai-image-sdk \
  --file release.yml --repo xinvxueyuan/NovelAI-Image-SDK \
  --env release --allow-stage-publish
```

Repeat for `@xinvxueyuan/novelai-image-sdk-{win32-x64-msvc,linux-x64-gnu,linux-arm64-gnu,darwin-arm64,darwin-x64}`.

**The first publish cannot use OIDC.** npm requires a package to already exist
before a trusted publisher can be configured for it, and refuses to create one
through trusted publishing ([npm/cli#8544][8544], still open). Bootstrap each
package once, locally, with an interactive 2FA login:

[8544]: https://github.com/npm/cli/issues/8544

```bash
# 1. Collect all five addons. The `npm` build job produces one per target, so a
#    dry run is enough — download `node-<target>` artifacts into bindings/node.
cd bindings/node
ls *.node                       # should list five: <binary>.<platformArchABI>.node

# 2. Log in once. This is the only step that prompts for a password or a
#    one-time code; everything after it is plain `npm publish` under that session.
npm login

# 3. Generate the platform manifests and file each addon into its directory.
npx napi create-npm-dirs
for addon in *.node; do
  suffix="${addon#*.}"; suffix="${suffix%.node}"
  mv "$addon" "npm/$suffix/"
done

# 4. Publish the platforms first: the root package's `optionalDependencies`
#    name them, and npm will not resolve a root package before they exist.
for dir in npm/*/; do npm publish "$dir" --access public; done
npm publish --access public
```

Deliberately **not** `napi pre-publish`: in napi-rs v3 that command runs
`npm publish` itself and creates a GitHub release, so it would bypass this
ordering and, on later releases, the staging review.

After that publish the trusted publishers can be registered (a newly created one
must complete its first successful publish within two days, or it expires and
has to be recreated), and every later release stages through CI with no
credential at all.

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
| `novelai_image_sdk-*.whl` | `python-wheels` → `pypi` | PyPI (trusted publishing, OIDC) |
| `*.node` addons, six packages | `npm` → `npm-publish` | npm (trusted publishing, staged for review) |
| Kotlin JAR + natives | `kotlin` / `kotlin-native` → `maven` | Maven Central |
| `NovelAIImageSDK.xcframework.zip` | `swift` | GitHub release; the checksum goes into `bindings/swift/Package.swift` |
| Release notes | `github-release` | GitHub release |

### Known gaps

- **Swift**: `Package.swift` carries a placeholder checksum until the first
  XCFramework is published; replace it with the value the `swift` job prints.
- **PyPI re-upload**: a version is immutable. Fix a bad release with a patch
  version, never by re-uploading the same one.
