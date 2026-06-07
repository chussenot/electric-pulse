#!/bin/sh
# Render every showcase ABC demo and print deterministic metrics.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

# shellcheck disable=SC2086
cc_build "$BIN_DIR/render-demos" tests/render_demos.c $ENGINE_SRC
"$BIN_DIR/render-demos"
