#!/usr/bin/env python3

"""Copy the VESC native binary into the package payload slot."""

from __future__ import annotations

import shutil
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: conv.py <native-binary> <package-binary>", file=sys.stderr)
        return 2

    native_binary = Path(sys.argv[1])
    package_binary = Path(sys.argv[2])
    package_binary.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(native_binary, package_binary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
