"""Retag the built wheel for distribution.

The native artifact is a cdylib loaded through ctypes, not a CPython extension
module, so one wheel per platform serves every Python 3 version.
setuptools-rust still tags it with the building interpreter's ABI; this script
rewrites that to `py3-none`.

The Linux platform tag is *not* handled here. PyPI rejects `linux_x86_64`, and
a compilable claim of manylinux has to be checked against the ELF symbols, so
the release workflow runs `auditwheel repair` afterwards.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

DIST = Path(__file__).resolve().parent.parent / "dist"

def retag(wheel: Path) -> int:
    command = [
        sys.executable,
        "-m",
        "wheel",
        "tags",
        str(wheel),
        "--python-tag",
        "py3",
        "--abi-tag",
        "none",
    ]
    command.append("--remove")
    return subprocess.run(command, check=False).returncode


def main() -> int:
    wheels = sorted(DIST.glob("*.whl"))
    if not wheels:
        print("no wheel built", file=sys.stderr)
        return 1
    for wheel in wheels:
        if retag(wheel) != 0:
            return 1
    print("retagged:", ", ".join(sorted(p.name for p in DIST.glob("*.whl"))))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
