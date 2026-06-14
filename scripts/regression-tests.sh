#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FIXTURE_DIR="$ROOT_DIR/experiment/mircap/tests/fixtures"

# 1. Build mirtool in release mode for speed
printf 'Building mirtool in release mode...\n'
cargo build --manifest-path "$ROOT_DIR/experiment/mirtool/Cargo.toml" --release > /dev/null

MIRTOOL="$ROOT_DIR/experiment/mirtool/target/release/mirtool"

# Check CC
CC_AVAILABLE=0
if command -v cc > /dev/null 2>&1; then
    CC_AVAILABLE=1
fi

# Check Upstream MIR
M2B_PATH="/home/john/project/mir-preservation/git/mir-restored/m2b"
MIR_BIN_RUN_PATH="/home/john/project/mir-preservation/git/mir-restored/mir-bin-run"
UPSTREAM_AVAILABLE=0
if [ -x "$M2B_PATH" ] && [ -x "$MIR_BIN_RUN_PATH" ]; then
    UPSTREAM_AVAILABLE=1
fi

# Print status header
printf '==================================================\n'
printf '   MIR-RETRODOC REGRESSION & DIFFERENTIAL TESTS   \n'
printf '==================================================\n'
printf 'C Transpiler Diff (cc):   %s\n' "$(if [ "$CC_AVAILABLE" -eq 1 ]; then echo "ENABLED"; else echo "DISABLED"; fi)"
printf 'Upstream MIR Diff (m2b):  %s\n' "$(if [ "$UPSTREAM_AVAILABLE" -eq 1 ]; then echo "ENABLED"; else echo "DISABLED"; fi)"
printf '==================================================\n\n'

# Find all valid and trap fixtures
FIXTURES=$(find "$FIXTURE_DIR" -type f \( -name "valid_*.mircap.txt" -o -name "trap_*.mircap.txt" \) | sort)

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

printf '%-40s | %-12s | %-12s | %-12s\n' "Fixture Name" "Interpreter" "C Transpiler" "Upstream MIR"
printf '%-40s-+-%-12s-+-%-12s-+-%-12s\n' "----------------------------------------" "------------" "------------" "------------"

for fixture in $FIXTURES; do
    name=$(basename "$fixture")
    
    # 1. Interpreter check
    interp_status="PASS"
    if ! "$MIRTOOL" run "$fixture" > /dev/null 2>&1; then
        interp_status="FAIL"
    fi
    
    # 2. C Transpiler check
    c_status="SKIP"
    if [ "$CC_AVAILABLE" -eq 1 ]; then
        if "$MIRTOOL" diff "$fixture" 2>&1 | grep -q "^PASS"; then
            c_status="PASS"
        else
            c_status="FAIL"
        fi
    fi
    
    # 3. Upstream MIR check
    upstream_status="SKIP"
    if [ "$UPSTREAM_AVAILABLE" -eq 1 ]; then
        if "$MIRTOOL" diff-upstream "$fixture" 2>&1 | grep -q "^PASS"; then
            upstream_status="PASS"
        else
            upstream_status="FAIL"
        fi
    fi
    
    # Determine overall status
    if [ "$interp_status" = "FAIL" ] || [ "$c_status" = "FAIL" ] || [ "$upstream_status" = "FAIL" ]; then
        FAIL_COUNT=$((FAIL_COUNT + 1))
    else
        if [ "$c_status" = "SKIP" ] && [ "$upstream_status" = "SKIP" ]; then
            SKIP_COUNT=$((SKIP_COUNT + 1))
        else
            PASS_COUNT=$((PASS_COUNT + 1))
        fi
    fi
    
    printf '%-40s | %-12s | %-12s | %-12s\n' "$name" "$interp_status" "$c_status" "$upstream_status"
done

printf '\n==================================================\n'
printf 'Summary: %d Passed, %d Failed, %d Skipped\n' "$PASS_COUNT" "$FAIL_COUNT" "$SKIP_COUNT"
printf '==================================================\n'

if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
