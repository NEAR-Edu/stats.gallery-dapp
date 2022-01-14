#!/usr/bin/env bash

wasm_path="$(find ./target/wasm32-unknown-unknown/release/ -maxdepth 1 -name "*.wasm")"

near deploy \
  --wasmFile $wasm_path \
  --accountId "$1" \
  --initFunction new \
  --initArgs "$(node ./init-args.js)"
