"""Retag the built wheel as `py3-none-<platform>`.

The native artifact is a cdylib loaded through ctypes, not a CPython
extension module, so one wheel per platform serves every Python 3 version.
setuptools-rust still tags it with the building interpreter's ABI.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

DIST = Path(__file__).resolve().parent.parent / "dist"


def main() -> int:
    wheels = sorted(DIST.glob("*.whl"))
    if not wheels:
        print("no wheel built", file=sys.stderr)
        return 1
    for wheel in wheels:
        completed = subprocess.run(
            [
                sys.executable,
                "-m",
                "wheel",
                "tags",
                str(wheel),
                "--python-tag",
                "py3",
                "--abi-tag",
                "none",
                "--remove",
            ],
            check=False,
        )
        if completed.returncode != 0:
            return completed.returncode
    print("retagged:", ", ".join(sorted(p.name for p in DIST.glob("*.whl"))))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
