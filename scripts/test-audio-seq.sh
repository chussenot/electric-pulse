#!/bin/sh
# Build and run the sequencer regression tests.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

# shellcheck disable=SC2086
cc_build "$BIN_DIR/test-audio-seq" tests/test_audio_seq.c $ENGINE_SRC
echo "Running sequencer regression tests..."
"$BIN_DIR/test-audio-seq"
