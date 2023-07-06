#! /usr/bin/env python3

import sys
import toml
from pathlib import Path
from typing import Any


def get_cargo_version(cargo: dict[str, Any]) -> str:
    return cargo["package"]["version"]


def get_pyproject_version(pyproject: dict[str, Any]) -> str:
    return pyproject["project"]["version"]


FILES = {
    Path(__file__).resolve().parent / "kuiper_cli" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent / "kuiper_lang" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent / "kuiper_python" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent / "kuiper_python" / "pyproject.toml": get_pyproject_version,
}


def main() -> None:
    versions = set()

    for file in FILES:
        with open(file, "r") as f:
            version = FILES[file](toml.load(f))
            print(f"{file}: {version}")
            versions.add(version)

    print()
    if len(versions) == 1:
        print(f"All versions are {versions.pop()}")
    else:
        print(f"Multiple version numbers found: {versions}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
