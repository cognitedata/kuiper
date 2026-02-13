#! /bin/bash
set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

if [ ! -f "$SCRIPT_DIR/../../target/release/libkuiper_interop.so" ]; then
    cargo build --release --package kuiper_interop
fi

cp "$SCRIPT_DIR"/../../target/release/libkuiper_interop.so "$SCRIPT_DIR"/
gcc -o "$SCRIPT_DIR"/test_kuiper_interop "$SCRIPT_DIR"/test_kuiper_interop.c -L"$SCRIPT_DIR" -lkuiper_interop -ldl

LD_LIBRARY_PATH="$SCRIPT_DIR" "$SCRIPT_DIR"/test_kuiper_interop
