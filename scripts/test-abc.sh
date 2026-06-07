#!/bin/sh
# Build and run the ABC parser + showcase regression tests.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

# shellcheck disable=SC2086
cc_build "$BIN_DIR/test-abc" tests/test_abc.c $ENGINE_SRC
echo "Running ABC parser tests..."
"$BIN_DIR/test-abc"
