#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
MIRTOOL_MANIFEST="$ROOT_DIR/experiment/mirtool/Cargo.toml"
FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/valid_data_segment_load.mircap.txt"
TRAP_FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/trap_load_oob.mircap.txt"
NO_CC=0

usage() {
    printf '%s\n' "usage: $0 [--no-cc]"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-cc)
            NO_CC=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
done

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

run() {
    printf '\n%s\n' "$ $*"
    "$@"
}

mirtool() {
    cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- "$@"
}

have_cc() {
    command -v cc >/dev/null 2>&1
}

printf '%s\n' "mir-retrodoc demo"
printf '%s\n' "fixture: $FIXTURE"
printf '%s\n' "temporary output: $TMP_DIR"

run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$FIXTURE"

run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- run "$FIXTURE"

printf '\n%s\n' "== MIR-F1 compile plan =="
mirtool plan "$FIXTURE" | sed -n '1,28p'

printf '\n%s\n' "== MIR-F1 lowered projection =="
mirtool lower "$FIXTURE" | sed -n '1,28p'

BIN_FILE="$TMP_DIR/demo.mircap"
C_FILE="$TMP_DIR/demo.c"

run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- encode "$FIXTURE" "$BIN_FILE" --force
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$BIN_FILE"

if [ "$NO_CC" -eq 1 ]; then
    printf '\n%s\n' "Skipping C compile and differential check because --no-cc was passed."
elif have_cc; then
    run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- compile-c "$FIXTURE" "$C_FILE"
    printf '\n%s\n' "generated C preview:"
    sed -n '1,24p' "$C_FILE"
    run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- diff "$FIXTURE"
else
    printf '\n%s\n' "Skipping C compile and differential check because 'cc' is not available."
fi

printf '\n%s\n' "== Trap case =="
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$TRAP_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- run "$TRAP_FIXTURE"

printf '\n%s\n' "demo complete"
