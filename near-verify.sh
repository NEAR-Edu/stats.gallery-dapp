#!/usr/bin/env bash

cargo build --target wasm32-unknown-unknown --release

WASM_PATH="$(find ./target/wasm32-unknown-unknown/release/ -maxdepth 1 -name "*.wasm" | head -1)"

cp $WASM_PATH "$1"
