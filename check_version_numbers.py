#! /usr/bin/env python3

import json
import re
import sys
from abc import ABC, abstractmethod
from io import TextIOWrapper
from pathlib import Path

import toml


def replace_in_file(file_name: str, src: str, target: str) -> None:
    with open(file_name, "r") as file:
        contents = file.read()

    contents = re.sub(src, target, contents)

    with open(file_name, "w") as file:
        file.write(contents)


class FileType(ABC):
    @abstractmethod
    def get_version(self, file: TextIOWrapper) -> str:
        pass

    @abstractmethod
    def set_version(self, file_name: str, version: str) -> None:
        pass


class Cargo(FileType):
    def get_version(self, file: TextIOWrapper) -> str:
        return toml.load(file)["package"]["version"]

    def set_version(self, file_name: str, version: str) -> None:
        replace_in_file(
            file_name,
            r"version = \"[0-9\.]+\"\nedition = \"2021\"",
            rf"version = \"{version}\"\nedition = \"2021\"",
        )


class PyProject(FileType):
    def get_version(self, file: TextIOWrapper) -> str:
        return toml.load(file)["project"]["version"]

    def set_version(self, file_name: str, version: str) -> None:
        replace_in_file(
            file_name,
            r"version = \"[0-9\.]+\"\ndescription =",
            rf"version = \"{version}\"\ndescription =",
        )


class JsPackage(FileType):
    def get_version(self, file: TextIOWrapper) -> str:
        return json.load(file)["version"]

    def set_version(self, file_name: str, version: str) -> None:
        replace_in_file(
            file_name, r"\"version\": \"[0-9\.]+\",", rf"\"version\": \"{version}\","
        )


FILES: dict[Path, FileType] = {
    Path(__file__).resolve().parent / "kuiper_cli" / "Cargo.toml": Cargo(),
    Path(__file__).resolve().parent / "kuiper_lang" / "Cargo.toml": Cargo(),
    Path(__file__).resolve().parent / "kuiper_python" / "Cargo.toml": Cargo(),
    Path(__file__).resolve().parent / "kuiper_python" / "pyproject.toml": PyProject(),
    Path(__file__).resolve().parent / "kuiper_lezer" / "package.json": JsPackage(),
    Path(__file__).resolve().parent / "kuiper_js" / "Cargo.toml": Cargo(),
    Path(__file__).resolve().parent / "kuiper_lang_macros" / "Cargo.toml": Cargo(),
}


def main() -> None:
    versions = set()

    if len(sys.argv) > 1:
        print(f"Setting version to {sys.argv[1]}")
        version = sys.argv[1]

        for file in FILES:
            FILES[file].set_version(file, version)
    else:
        for file in FILES:
            with open(file, "r") as f:
                version = FILES[file].get_version(f)
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
