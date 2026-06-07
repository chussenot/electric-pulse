#!/bin/sh
# Build and run the audio DSP regression tests.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

cc_build "$BIN_DIR/test-audio-dsp" tests/test_audio_dsp.c src/audio_dsp.c
echo "Running audio DSP regression tests..."
"$BIN_DIR/test-audio-dsp"
