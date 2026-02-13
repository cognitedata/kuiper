#! /bin/bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
LIB_DIR="$SCRIPT_DIR/../../target/release"

if [ ! -f "$LIB_DIR/libkuiper_interop.so" ]; then
    cargo build --release --package kuiper_interop
fi

gcc -Wall -Wextra -Werror -o "$SCRIPT_DIR"/test_kuiper_interop "$SCRIPT_DIR"/test_kuiper_interop.c -L"$LIB_DIR" -lkuiper_interop -ldl
LD_LIBRARY_PATH="$LIB_DIR" "$SCRIPT_DIR"/test_kuiper_interop
