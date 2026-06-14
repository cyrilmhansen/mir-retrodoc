#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

CRATES="mircap mirsem mirc0 mirtool mirspace mirplan mirjit mirrv32"
MODE=${1:-}

case "$MODE" in
    ""|"--check")
        ;;
    *)
        printf '%s\n' "usage: $0 [--check]" >&2
        exit 2
        ;;
esac

for crate in $CRATES; do
    if [ "$MODE" = "--check" ]; then
        printf '%s\n' "==> cargo fmt --check ($crate)"
        (cd "$ROOT_DIR/experiment/$crate" && cargo fmt --check)
    else
        printf '%s\n' "==> cargo fmt ($crate)"
        (cd "$ROOT_DIR/experiment/$crate" && cargo fmt)
    fi
done
