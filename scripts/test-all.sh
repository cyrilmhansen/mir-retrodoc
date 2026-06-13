#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

CRATES="mircap mirsem mirc0 mirtool mirspace mirplan"

for crate in $CRATES; do
    printf '%s\n' "==> cargo test ($crate)"
    (cd "$ROOT_DIR/experiment/$crate" && cargo test)
done
