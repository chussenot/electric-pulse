#!/bin/sh
# Run the engine's own tests (Sonomorph's chip8, at the pinned submodule
# commit): its C regression tests for the DSP, sequencer and FX, and its
# render, song, jam and MIDI contracts. They moved there with the engine.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh
require_engine

cargo test --manifest-path vendor/sonomorph/Cargo.toml --features jam,midi
