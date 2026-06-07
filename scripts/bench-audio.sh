#!/bin/sh
# Build and run the audio microbenchmark.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

cc_build "$BIN_DIR/bench-audio" tests/bench_audio.c src/audio_dsp.c
echo "Build complete: $BIN_DIR/bench-audio"
"$BIN_DIR/bench-audio"
