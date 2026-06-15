#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

step() {
    printf '\n==> %s\n' "$1"
}

step "format check"
"$ROOT_DIR/scripts/fmt-all.sh" --check

step "full test suite"
"$ROOT_DIR/scripts/test-all.sh"

step "demo smoke test"
"$ROOT_DIR/scripts/demo.sh" --no-pause

step "git whitespace check"
(cd "$ROOT_DIR" && git diff --check)

printf '\n%s\n' "precommit checks passed"
