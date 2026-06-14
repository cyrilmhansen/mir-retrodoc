#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

cargo run --quiet --manifest-path "$ROOT_DIR/experiment/mirjit/Cargo.toml" --bin demo
