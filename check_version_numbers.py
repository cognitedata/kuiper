#! /usr/bin/env python3

from io import TextIOWrapper
import sys
import toml
import json
from pathlib import Path


def get_cargo_version(file: TextIOWrapper) -> str:
    return toml.load(file)["package"]["version"]


def get_pyproject_version(file: TextIOWrapper) -> str:
    return toml.load(file)["project"]["version"]


def get_js_package_version(file: TextIOWrapper) -> str:
    return json.load(file)["version"]


FILES = {
    Path(__file__).resolve().parent / "kuiper_cli" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent / "kuiper_lang" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent / "kuiper_python" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent
    / "kuiper_python"
    / "pyproject.toml": get_pyproject_version,
    Path(__file__).resolve().parent
    / "kuiper_lezer"
    / "package.json": get_js_package_version,
    Path(__file__).resolve().parent / "kuiper_js" / "Cargo.toml": get_cargo_version,
    Path(__file__).resolve().parent
    / "kuiper_lang_macros"
    / "Cargo.toml": get_cargo_version,
}


def main() -> None:
    versions = set()

    for file in FILES:
        with open(file, "r") as f:
            version = FILES[file](f)
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
