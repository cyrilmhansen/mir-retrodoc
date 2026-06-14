#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
MIRTOOL_MANIFEST="$ROOT_DIR/experiment/mirtool/Cargo.toml"
FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/valid_data_segment_load.mircap.txt"
TRAP_FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/trap_load_oob.mircap.txt"
NO_CC=0
PAUSE=1

usage() {
    printf '%s\n' "usage: $0 [--no-cc] [--no-pause]"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --no-cc)
            NO_CC=1
            shift
            ;;
        --no-pause)
            PAUSE=0
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

pause() {
    if [ "$PAUSE" -eq 1 ]; then
        printf '\n%s' "Press Enter to continue..."
        read _answer || true
        printf '\n'
    fi
}

section() {
    printf '\n%s\n' "================================================================"
    printf '%s\n' "$1"
    printf '%s\n' "================================================================"
}

explain() {
    printf '\n%s\n' "$1"
}

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

cat_file() {
    printf '\n%s\n' "$ cat $1"
    cat "$1"
}

section "mir-retrodoc demo"
explain "This tour shows the current F0/F1 boundary: a tiny validated MIR-inspired subset, an interpreter oracle, C differential testing, and F1 planning artifacts for future backends."
explain "The script uses temporary output under: $TMP_DIR"
pause

section "Step 1: inspect the valid MIR-F0 fixture"
explain "This file is the human-readable input. It defines types, symbols, one data segment, one function, blocks, and instructions."
explain "The important detail for the demo is the data segment at offset 100. The program loads byte 1 from it, so the expected result is u32 43."
cat_file "$FIXTURE"
pause

section "Step 2: validate the fixture"
explain "MIR-F0 validation is conservative. Unsupported or malformed constructs are rejected before interpretation or compilation."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$FIXTURE"
pause

section "Step 3: run with mirsem"
explain "mirsem is the reference interpreter. It is the semantic oracle used by tests and differential checking."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- run "$FIXTURE"
pause

section "Step 4: inspect the MIR-F1 compile plan"
explain "mirtool plan builds ProgramSpace and CompilePlan. This is not code generation; it is a deterministic compiler-facing description of functions, blocks, instructions, data, calls, and memory operations."
printf '\n%s\n' "$ mirtool plan $FIXTURE | sed -n '1,40p'"
mirtool plan "$FIXTURE" | sed -n '1,40p'
pause

section "Step 5: inspect the MIR-F1 lowered projection"
explain "mirtool lower projects the plan into a backend-facing shape: explicit reads, writes, branch targets, direct calls, memory operations, and data segment summaries."
explain "This is the current candidate contract for future backends. It deliberately avoids choosing RISC-V, fantasy-computer details, register allocation, or optimization."
printf '\n%s\n' "$ mirtool lower $FIXTURE | sed -n '1,40p'"
mirtool lower "$FIXTURE" | sed -n '1,40p'
pause

BIN_FILE="$TMP_DIR/demo.mircap"
C_FILE="$TMP_DIR/demo.c"

section "Step 6: encode to Cap'n Proto and validate the binary path"
explain "The same immutable ModuleImage can be loaded from text or from the Cap'n Proto binary encoding. F1 tests assert that plan and lowered artifacts are identical across both load paths."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- encode "$FIXTURE" "$BIN_FILE" --force
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$BIN_FILE"
pause

section "Step 7: compile to C and compare against the interpreter"
if [ "$NO_CC" -eq 1 ]; then
    explain "Skipping C compile and differential check because --no-cc was passed."
elif have_cc; then
    explain "mirc0 is the current C backend. mirtool diff runs mirsem first, compiles generated C with cc, executes it, and compares the observable result."
    run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- compile-c "$FIXTURE" "$C_FILE"
    printf '\n%s\n' "Generated C preview:"
    printf '%s\n' "$ sed -n '1,36p' $C_FILE"
    sed -n '1,36p' "$C_FILE"
    run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- diff "$FIXTURE"
else
    explain "Skipping C compile and differential check because 'cc' is not available."
fi
pause

section "Step 8: inspect and run a trap case"
explain "F0 traps are part of the contract. This fixture validates structurally, then traps at runtime with an out-of-bounds load."
cat_file "$TRAP_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$TRAP_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- run "$TRAP_FIXTURE"
pause

section "Future direction"
explain "Near-term F1 work should keep proving the lowered contract before adding new semantics."
printf '%s\n' "- make the experimental lowered C path cover more F0 fixtures"
printf '%s\n' "- decide whether mirc0 should eventually consume LoweredProgram by default"
printf '%s\n' "- add a small backend trait once the lowered contract stops moving"
printf '%s\n' "- only then choose the first target-facing feature, such as RISC-V32 or a fantasy-computer backend"
printf '%s\n' "- defer i64 helpers, floats, host ABI, optimization, and runtime replacement until the backend boundary is stable"

section "demo complete"
