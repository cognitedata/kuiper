#! /bin/env python
import sys
from pathlib import Path
from typing import TextIO, Any

import yaml


def generate_docs(functions: list[dict[str, Any]], file: TextIO):
    file.write(
        """---
pagination_next: null
pagination_prev: null
title: Functions
---

# Functions
"""
    )
    for function in functions:
        file.write("\n")

        file.write(f"## {function['name'].strip()}\n\n")
        file.write(f"{function['signature'].strip()}\n\n")
        file.write(f"{function['description'].strip()}\n\n")
        file.write(f"**Code example{'s' if len(function['examples']) > 1 else ''}**\n")
        for example in function["examples"]:
            file.write("```\n")
            file.write(example.strip())
            file.write("\n```\n")


def generate_warning_header(file: TextIO, comment_tag="//"):
    file.write(
        f"{comment_tag} This file is automatically created by kuiper_documentation/codegen.py. Do not edit it directly.\n"
    )
    file.write(f"{comment_tag}\n")
    file.write(
        f"{comment_tag} To change the content of this file, edit kuiper_documentation/functions.yaml instead.\n\n"
    )


def generate_repl_list(functions: list[dict[str, Any]], file: TextIO):
    generate_warning_header(file)

    file.write(f"pub const BUILT_INS: [&str; {len(functions)+1}] = [\n")
    for function in functions:
        file.write(f"    \"{function['name'].strip()}(\",\n")
    file.write('    "input",\n')
    file.write("];\n")


def generate_js_list(functions: list[dict[str, Any]], file: TextIO):
    generate_warning_header(file)

    file.write(
        """export type KuiperInput = {
    label: string,
    description: string,
};\n\n"""
    )

    file.write("export const builtIns: KuiperInput[] = [\n")

    for function in functions:
        short_desc = function["description"].split("\n")[0].strip()
        file.write(
            f'    {{ label: "{function["name"].strip()}", description: "{function["signature"].strip()}: {short_desc}" }},\n'
        )

    file.write("];\n")


def find_function_defs(file: TextIO) -> set[str]:
    names = set()

    in_func = False
    in_match = False

    # Very nasty, don't look
    for line in file:
        if "get_function_expression" in line:
            in_func = True
            continue

        if not in_func:
            continue

        if "match name" in line:
            in_match = True
            continue

        if not in_match:
            continue

        if line.strip() == "};":
            break

        if "=>" not in line:
            continue

        name = line.split("=>")[0].strip().strip('"')
        if name != "_":
            names.add(name)

    return names


def main():
    project_base = Path(sys.path[0]).parent

    with open(project_base / "kuiper_documentation" / "functions.yaml") as f:
        functions: list[dict[str, Any]] = yaml.safe_load(f)["functions"]

    functions.sort(key=lambda function: function["name"])

    function_names = {function["name"] for function in functions}
    with open(project_base / "kuiper_lang" / "src" / "expressions" / "base.rs") as f:
        true_function_names = find_function_defs(f)

    return_val = 0

    for function in true_function_names:
        if function not in function_names:
            print(f"Missing documentation for {function}", file=sys.stderr)
            return_val = 1
    for function in function_names:
        if function not in true_function_names:
            print(
                f"Function {function} is documented, but doesn't exist", file=sys.stderr
            )
            return_val = 1

    with open(
        project_base / "kuiper_documentation" / "built_in_functions.md", "w"
    ) as f:
        generate_docs(functions, f)
    with open(project_base / "kuiper_cli" / "src" / "builtins.rs", "w") as f:
        generate_repl_list(functions, f)
    with open(project_base / "kuiper_lezer" / "src" / "builtins.ts", "w") as f:
        generate_js_list(functions, f)

    return return_val


if __name__ == "__main__":
    sys.exit(main())
