#!/bin/sh
# Export a demo/song to a Standard MIDI File via the read-only SeqSong bridge.
# This is OFF the render path (ADR-0003): it never invokes the audio engine.
# Usage: scripts/export-midi.sh <name|path.abc> [out.mid]
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

NAME="${1:-}"
OUT="${2:-}"

if [ -z "$NAME" ]; then
	echo "Usage: mise run export-midi <name|path.abc> [out.mid]" >&2
	echo "Examples:" >&2
	echo "  mise run export-midi glass_anthem" >&2
	echo "  mise run export-midi glass_anthem /tmp/glass.mid" >&2
	exit 1
fi

# shellcheck disable=SC2086
cc_build "$BIN_DIR/export-midi" tests/export_midi.c src/midi_export.c $ENGINE_SRC

if [ -n "$OUT" ]; then
	"$BIN_DIR/export-midi" "$NAME" "$OUT"
else
	"$BIN_DIR/export-midi" "$NAME"
fi
