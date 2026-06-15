#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
MIRTOOL_MANIFEST="$ROOT_DIR/experiment/mirtool/Cargo.toml"
FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/valid_data_segment_load.mircap.txt"
DIRECT_CALL_FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/valid_direct_call.mircap.txt"
FLOAT_FIXTURE="$ROOT_DIR/experiment/mircap/tests/fixtures/valid_float_arithmetic.mircap.txt"
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

section "Step 4: analyze static function effects"
explain "mirtool analyze is the first reflection-oriented slice. It reports conservative per-function facts such as allocation, memory effects, traps, direct calls, CFG acyclicity, trivial termination, and pure-candidate status."
explain "This is intentionally structural today. Future work can compare these static summaries with mirsem traces and runtime performance counters."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- analyze "$FIXTURE"
explain "mirtool trace-check now performs that first comparison for one run: static facts beside observed mirsem counters."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- trace-check "$FIXTURE"
explain "Both reports also have JSON output for tests, dashboards, IDE tooling, or future runtime monitors. Here is a compact preview of the trace-check contract."
printf '\n%s\n' "$ mirtool trace-check $FIXTURE --json | cut -c1-220"
mirtool trace-check "$FIXTURE" --json | cut -c1-220
explain "For direct calls, trace-check compares static direct-call edges with observed caller/callee counts."
printf '\n%s\n' "$ mirtool trace-check $DIRECT_CALL_FIXTURE | sed -n '1,22p'"
mirtool trace-check "$DIRECT_CALL_FIXTURE" | sed -n '1,22p'
explain "mirtool cost adds the first symbolic cost summary over the lowered plan. Acyclic functions get structural counts; cyclic CFGs are marked unknown instead of overclaiming complexity."
printf '\n%s\n' "$ mirtool cost $FIXTURE"
mirtool cost "$FIXTURE"
printf '\n%s\n' "$ mirtool cost $DIRECT_CALL_FIXTURE --json | cut -c1-220"
mirtool cost "$DIRECT_CALL_FIXTURE" --json | cut -c1-220
pause

section "Step 5: inspect the MIR-F1 compile plan"
explain "mirtool plan builds ProgramSpace and CompilePlan. This is not code generation; it is a deterministic compiler-facing description of functions, blocks, instructions, data, calls, and memory operations."
printf '\n%s\n' "$ mirtool plan $FIXTURE | sed -n '1,40p'"
mirtool plan "$FIXTURE" | sed -n '1,40p'
pause

section "Step 6: inspect the MIR-F1 lowered projection"
explain "mirtool lower projects the plan into a backend-facing shape: explicit reads, writes, branch targets, direct calls, memory operations, and data segment summaries."
explain "This is the current candidate contract for future backends. It deliberately avoids choosing RISC-V, fantasy-computer details, register allocation, or optimization."
printf '\n%s\n' "$ mirtool lower $FIXTURE | sed -n '1,40p'"
mirtool lower "$FIXTURE" | sed -n '1,40p'
pause

BIN_FILE="$TMP_DIR/demo.mircap"
LARGE_FIXTURE="$TMP_DIR/unrolled_sum.mircap.txt"
LARGE_BIN_FILE="$TMP_DIR/unrolled_sum.mircap"
C_FILE="$TMP_DIR/demo.c"

section "Step 7: encode to Cap'n Proto and validate the binary path"
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

section "Step 8: Cap'n Proto binary view and load-time comparison"
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

section "Step 9: compile to C and compare against the interpreter"
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

section "Step 10: float arithmetic C differential path"
explain "The current float slice supports f32/f64 constants and arithmetic in mircap validation, mirsem execution, and mirc0 C differential testing."
explain "Results include both decimal text and the exact IEEE-754 bit pattern, which keeps the demo deterministic while comparisons/conversions remain unspecified."
cat_file "$FLOAT_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- run "$FLOAT_FIXTURE"
if [ "$NO_CC" -eq 1 ]; then
    explain "Skipping float C differential check because --no-cc was passed."
elif have_cc; then
    run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- diff "$FLOAT_FIXTURE"
else
    explain "Skipping float C differential check because 'cc' is not available."
fi
pause

section "Step 11: inspect and run a trap case"
explain "F0 traps are part of the contract. This fixture validates structurally, then traps at runtime with an out-of-bounds load."
cat_file "$TRAP_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- validate "$TRAP_FIXTURE"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- run "$TRAP_FIXTURE"
pause

section "Step 12: compile to RISC-V 32-bit Assembly (MIR-F1 candidate RV32I backend)"
explain "mirtool compile-rv32i compiles the module image to RISC-V assembly using the newly integrated RV32I backend with linear scan register allocation and spill handling."
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- compile-rv32i "$FIXTURE" "$TMP_DIR/demo.s"
printf '\n%s\n' "Generated RISC-V Assembly preview (first 40 lines):"
printf '%s\n' "$ head -n 40 $TMP_DIR/demo.s"
head -n 40 "$TMP_DIR/demo.s"
pause

explain "The RV32I backend also features full 64-bit integer (i64) lowering, splitting 64-bit operations to 32-bit register carry math and spilling 64-bit values to stack offsets."
DEMO_I64="$TMP_DIR/demo_i64.mircap.txt"
cat << 'EOF' > "$DEMO_I64"
mircap mircap
version 0
module 1 demo_i64
type 1 i32
type 2 i64
symbol 1 main function
function 1 1 - 2 3 0 2,2,2
func_block 1 1
block 1 1 1 2 3 4
insn 1 const_i64 r:0 l:100000000000
insn 2 const_i64 r:1 l:200000000000
insn 3 add_i64 r:2 v:0 v:1
insn 4 ret v:2
EOF
cat_file "$DEMO_I64"
run cargo run --quiet --manifest-path "$MIRTOOL_MANIFEST" -- compile-rv32i "$DEMO_I64" "$TMP_DIR/demo_i64.s"
printf '\n%s\n' "Generated 64-bit RISC-V Assembly:"
printf '%s\n' "$ cat $TMP_DIR/demo_i64.s"
cat "$TMP_DIR/demo_i64.s"
pause

section "Completed work & current status"
explain "The MIR-F1 experimental pipeline has successfully achieved:"
printf '%s\n' "- full workspace-wide support for 64-bit integers (i64) in mircap, mirsem, mirc0, mirrv32, mirjit, and mirtool"
printf '%s\n' "- f32/f64 constants and arithmetic validated in mircap, executed in mirsem, emitted by mirc0, and checked with C differential tests"
printf '%s\n' "- linear scan register allocation with callee-saved register spill handling in the RV32I backend"
printf '%s\n' "- target-neutral lowering and projection, tested using differential checks"
printf '%s\n' "- static effect summaries, trace-backed call-edge checks, symbolic cost summaries, and JSON reflection reports"

section "demo complete"
