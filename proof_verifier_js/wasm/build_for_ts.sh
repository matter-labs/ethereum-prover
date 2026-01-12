#!/usr/bin/env bash
set -e

RUST_MIN_STACK=267108864 wasm-pack build --target web --out-dir ../ts/wasm/pkg

# `wasm-pack` generates `.gitignore`, which prevents these files to be published to npm
# Right now, there is no flag to omit generating this file, so we just remove it here
# See: https://github.com/drager/wasm-pack/issues/728
rm ../ts/wasm/pkg/.gitignore
