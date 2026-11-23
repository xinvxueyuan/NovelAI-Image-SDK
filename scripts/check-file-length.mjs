// Rejects code files longer than 500 lines.
//
// With file arguments (the pre-commit hook passes staged paths) only those are
// checked; without arguments the whole tracked tree is. A file that grows past
// the limit is split by role: production code into modules, tests beside them.
// Physical lines count, so comments and blank lines do too.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

/// No code file may exceed this many lines.
export const MAX_LINES = 500;

/// `node script.mjs [file...]`: skip the interpreter and the script path.
const FIRST_FILE_ARGUMENT = 2;

/// Extensions that carry code this repository writes by hand.
const CODE_EXTENSIONS = [".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

const IGNORED_PATH_SEGMENTS = ["target/", "node_modules/", "dist/", ".turbo/"];

export function trackedFiles() {
  const output = execFileSync("git", ["ls-files"], { encoding: "utf8" });
  return output.split("\n").filter((line) => line.length > 0);
}

export function isCodeFile(path) {
  if (IGNORED_PATH_SEGMENTS.some((segment) => path.includes(segment))) {
    return false;
  }
  // Generated bindings are not hand-written.
  if (path.includes("/generated/")) {
    return false;
  }
  return CODE_EXTENSIONS.some((extension) => path.endsWith(extension));
}

export function countLines(path) {
  return readFileSync(path, "utf8").split("\n").length;
}

/// The code files in `files` that exceed the limit, longest first.
export function oversizedFiles(files, limit = MAX_LINES) {
  return files
    .filter(isCodeFile)
    .map((file) => ({ file, lines: countLines(file) }))
    .filter(({ lines }) => lines > limit)
    .sort((a, b) => b.lines - a.lines);
}

function main(argv) {
  const explicit = argv.slice(FIRST_FILE_ARGUMENT);
  const files = explicit.length === 0 ? trackedFiles() : explicit;
  const offenders = oversizedFiles(files);
  if (offenders.length === 0) {
    return 0;
  }
  console.error(
    `Code files must stay at or below ${MAX_LINES} lines; split by role:`,
  );
  for (const { file, lines } of offenders) {
    console.error(`  ${lines}  ${file}`);
  }
  return 1;
}

process.exitCode = main(process.argv);
