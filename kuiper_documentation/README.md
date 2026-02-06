# Documentation and code generation for Kuiper

This directory contains code and documentation generation for Kuiper.

The `functions.yaml` file contains a list of all available functions in Kuiper. This is used to generate several files
around the code base, such as the [builtins.rs](../kuiper_cli/src/builtins.rs) file for the Kuiper CLI, as well as the
[documentation for built-in functions](./built_in_functions.md) in this directory.

The [codegen.py](./codegen.py) script will take the `function.yaml` file and produce all the auto-generated files in
the repo. Whenever adding new functions to Kuiper, the `functions.yaml` file should be updated accordingly. This is
also verified as part of the automated build process.
