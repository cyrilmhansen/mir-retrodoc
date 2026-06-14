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

file_size() {
    wc -c < "$1" | tr -d ' '
}

cat_file() {
    printf '\n%s\n' "$ cat $1"
    cat "$1"
}

show_text_vs_hex() {
    text_file=$1
    binary_file=$2
    text_preview="$TMP_DIR/text-preview.txt"
    hex_preview="$TMP_DIR/hex-preview.txt"

    nl -ba "$text_file" | sed -n '1,16p' | awk '{ if (length($0) > 58) print substr($0, 1, 55) "..."; else print }' > "$text_preview"
    od -An -tx1 -v "$binary_file" | sed 's/^ *//' | sed -n '1,16p' > "$hex_preview"

    printf '\n%s\n' "Text fixture lines, truncated                             Cap'n Proto bytes as hex"
    printf '%s\n' "--------------------------------------------------------------------------------"
    paste "$text_preview" "$hex_preview"
}

show_generated_fixture_preview() {
    text_file=$1
    printf '\n%s\n' "$ sed -n '1,7p' $text_file"
    sed -n '1,7p' "$text_file"
    printf '\n%s\n' "Generated block line preview:"
    sed -n '8p' "$text_file" | awk '{ printf "%s...\n", substr($0, 1, 160) }'
    printf '\n%s\n' "$ sed -n '9,24p' $text_file"
    sed -n '9,24p' "$text_file"
    printf '\n%s\n' "$ tail -n 8 $text_file"
    tail -n 8 "$text_file"
}

generate_unrolled_sum_fixture() {
    output=$1
    count=$2
    insn_total=$((count * 2 + 2))

    {
        printf '%s\n' "mircap mircap"
        printf '%s\n' "version 0"
        printf '%s\n' "module 1 unrolled_sum"
        printf '%s\n' "type 1 u32"
        printf '%s\n' "symbol 1 main function"
        printf '%s\n' "function 1 1 - 1 2 0 1,1"
        printf '%s\n' "func_block 1 1"
        printf '%s' "block 1 1"
        insn_id=1
        while [ "$insn_id" -le "$insn_total" ]; do
            printf ' %s' "$insn_id"
            insn_id=$((insn_id + 1))
        done
        printf '\n'
        printf '%s\n' "insn 1 const_u32 r:0 u:0"

        insn_id=2
        value=1
        while [ "$value" -le "$count" ]; do
            printf 'insn %s const_u32 r:1 u:%s\n' "$insn_id" "$value"
            insn_id=$((insn_id + 1))
            printf 'insn %s add_u32 r:0 v:0 v:1\n' "$insn_id"
            insn_id=$((insn_id + 1))
            value=$((value + 1))
        done
        printf 'insn %s ret v:0\n' "$insn_id"
    } > "$output"
}

generate_large_fixture() {
    output=$1
    count=256
    generate_unrolled_sum_fixture "$output" "$count"
    while [ "$(file_size "$output")" -lt 20480 ]; do
        count=$((count + 32))
        generate_unrolled_sum_fixture "$output" "$count"
    done
    printf '%s\n' "$count"
}

bench_avg_ns() {
    mirtool bench-load "$1" --iterations 1000 | awk '/^avg_ns:/ { print $2 }'
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
LARGE_FIXTURE="$TMP_DIR/unrolled_sum.mircap.txt"
LARGE_BIN_FILE="$TMP_DIR/unrolled_sum.mircap"
C_FILE="$TMP_DIR/demo.c"

section "Step 6: encode to Cap'n Proto and validate the binary path"
explain "The same immutable ModuleImage can be loaded from text or from the Cap'n Proto binary encoding. First, the small hand-written fixture proves the binary path against the same example used earlier."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- encode "$FIXTURE" "$BIN_FILE" --force
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$BIN_FILE"
UNROLL_COUNT=$(generate_large_fixture "$LARGE_FIXTURE")
LARGE_TEXT_SIZE=$(file_size "$LARGE_FIXTURE")
explain "For the Cap'n Proto comparison, the demo now generates a longer algorithm: an unrolled u32 summation loop with $UNROLL_COUNT additions. The generated text is $LARGE_TEXT_SIZE bytes, close to 20 KiB."
show_generated_fixture_preview "$LARGE_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$LARGE_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- encode "$LARGE_FIXTURE" "$LARGE_BIN_FILE" --force
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$LARGE_BIN_FILE"
pause

section "Step 7: Cap'n Proto binary view and load-time comparison"
explain "The text format is useful for humans and fixtures. Cap'n Proto is the structured binary path intended for stable serialized module images and faster tool loading."
TEXT_SIZE=$(file_size "$LARGE_FIXTURE")
BIN_SIZE=$(file_size "$LARGE_BIN_FILE")
printf '\n%s\n' "Size comparison:"
printf '%s\n' "  text bytes:   $TEXT_SIZE"
printf '%s\n' "  binary bytes: $BIN_SIZE"
awk -v text="$TEXT_SIZE" -v bin="$BIN_SIZE" 'BEGIN { if (bin > 0) printf "  text/binary size ratio: %.2fx\n", text / bin }'
awk -v text="$TEXT_SIZE" -v bin="$BIN_SIZE" 'BEGIN { if (bin < text) print "  size note: binary is smaller for this generated fixture."; else print "  size note: binary is not smaller here; schema and framing overhead still dominate this shape." }'
show_text_vs_hex "$LARGE_FIXTURE" "$LARGE_BIN_FILE"
printf '\n%s\n' "In-process loading benchmark, 1000 iterations, excluding repeated file I/O and cargo startup:"
TEXT_AVG=$(bench_avg_ns "$LARGE_FIXTURE")
BIN_AVG=$(bench_avg_ns "$LARGE_BIN_FILE")
printf '%s\n' "  text avg_ns:   $TEXT_AVG"
printf '%s\n' "  binary avg_ns: $BIN_AVG"
awk -v text="$TEXT_AVG" -v bin="$BIN_AVG" 'BEGIN { if (bin > 0) printf "  text/binary load-time ratio: %.2fx\n", text / bin }'
awk -v text="$TEXT_AVG" -v bin="$BIN_AVG" 'BEGIN { if (text > bin) print "  load-time note: binary is faster on this run."; else print "  load-time note: binary was not faster on this run; rerun on the presentation machine before quoting a number." }'
explain "The exact ratios depend on the machine and generated program shape. The pedagogical point is that the project keeps readable source fixtures while proving a structured binary load path on a non-trivial module."
pause

section "Step 8: compile to C and compare against the interpreter"
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

section "Step 9: inspect and run a trap case"
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
