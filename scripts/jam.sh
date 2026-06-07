#!/bin/sh
# Build the infinite-continuation player (bin/electric-pulse-jam).
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

JAM_BIN="$BIN_DIR/electric-pulse-jam"
JAM_CFLAGS="-Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600 -Isrc"

mkdir -p "$BIN_DIR"
# shellcheck disable=SC2086
$CC $JAM_CFLAGS -o "$JAM_BIN" tools/electric_pulse_jam.c $ENGINE_SRC src/audio_jam.c -lm
