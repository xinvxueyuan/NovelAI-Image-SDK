// Public entry point for @xinvxueyuan/novelai-image-sdk.
//
// The native addon is produced by `napi build` into this directory (local
// development) or shipped as a platform package (published wheels). The
// generated `index.d.ts` describes the same surface.

"use strict";

const { existsSync, readdirSync } = require("node:fs");
const { join } = require("node:path");

function loadAddon() {
  const candidates = readdirSync(__dirname).filter((name) =>
    name.endsWith(".node"),
  );
  if (candidates.length === 0) {
    throw new Error(
      "No native addon found. Build it with `napi build --release` in " +
        "bindings/node, or install a prebuilt platform package.",
    );
  }

  // Prefer the artifact matching this platform/arch.
  const platform = process.platform;
  const arch = process.arch;
  const preferred = candidates.find(
    (name) => name.includes(platform) && name.includes(arch),
  );
  const chosen = preferred ?? candidates[0];

  const path = join(__dirname, chosen);
  if (!existsSync(path)) {
    throw new Error(`native addon ${chosen} is missing`);
  }
  return require(path);
}

module.exports = loadAddon();
