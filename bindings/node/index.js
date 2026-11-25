// Public entry point for @xinvxueyuan/novelai-image-sdk.
//
// The native addon arrives one of two ways:
//
//   1. Beside this file — `napi build` writes it here during local development.
//   2. As a platform package — `npm install` resolves the matching entry from
//      `optionalDependencies`, and the addon lives in that package's directory.
//
// The generated `index.d.ts` describes the surface either way.

"use strict";

const { existsSync, readdirSync } = require("node:fs");
const { join } = require("node:path");

const SCOPE = "@xinvxueyuan/novelai-image-sdk";

// Must stay in step with `napi.targets` in package.json.
const SUFFIXES = [
  "win32-x64-msvc",
  "linux-x64-gnu",
  "linux-arm64-gnu",
  "darwin-arm64",
  "darwin-x64",
];

/** The `<platform>-<arch>[-<abi>]` suffix this process can load. */
function hostSuffix() {
  const arch = process.arch;
  switch (process.platform) {
    case "win32":
      return arch === "x64" ? "win32-x64-msvc" : null;
    case "darwin":
      return arch === "arm64" ? "darwin-arm64" : "darwin-x64";
    case "linux":
      if (arch === "x64") return "linux-x64-gnu";
      if (arch === "arm64") return "linux-arm64-gnu";
      return null;
    default:
      return null;
  }
}

/** Load the addon that sits next to this file, if a build put one there. */
function fromThisDirectory(suffix) {
  const names = readdirSync(__dirname).filter((name) =>
    name.endsWith(".node"),
  );
  if (names.length === 0) return null;

  const preferred = names.find(
    (name) => suffix && name.includes(suffix) && name.startsWith("novelai"),
  );
  const chosen = preferred ?? names.find((name) => name.startsWith("novelai"));
  if (!chosen) return null;

  const path = join(__dirname, chosen);
  return existsSync(path) ? require(path) : null;
}

/** Load the addon from the platform package npm installed beside us. */
function fromPlatformPackage(suffix) {
  if (!suffix) return null;
  try {
    return require(`${SCOPE}-${suffix}`);
  } catch (error) {
    // A package that is absent for this platform is expected: npm only
    // installs the matching entry from `optionalDependencies`.
    if (error && error.code === "MODULE_NOT_FOUND") return null;
    throw error;
  }
}

function loadAddon() {
  const suffix = hostSuffix();
  const addon = fromThisDirectory(suffix) ?? fromPlatformPackage(suffix);
  if (addon) return addon;

  throw new Error(
    `No native addon for ${process.platform}/${process.arch}. ` +
      `Expected one of: ${SUFFIXES.join(", ")}. ` +
      "Install a prebuilt platform package, or run `napi build --release` " +
      "in bindings/node.",
  );
}

module.exports = loadAddon();
