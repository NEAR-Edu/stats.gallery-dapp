#!/usr/bin/env bash

wasm_path="$(find ./target/wasm32-unknown-unknown/release/ -maxdepth 1 -name "*.wasm")"

near dev-deploy \
  --wasmFile $wasm_path \
  "$@"

near call "$(<./neardev/dev-account)" new "$(node ./init-args.js)" --accountId "$(<./neardev/dev-account)"
