#!/bin/sh
# Render a single demo and optionally export a WAV.
# Usage: scripts/play-demo.sh <demo> [wav]
#   or:  DEMO=<demo> WAV=1 scripts/play-demo.sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

DEMO="${1:-${DEMO:-}}"
WAV="${2:-${WAV:-}}"

if [ -z "$DEMO" ]; then
	echo "Usage: mise run play-demo <demo> [wav]" >&2
	echo "Examples:" >&2
	echo "  mise run play-demo dark_moroder" >&2
	echo "  mise run play-demo neon_nightdrive wav" >&2
	exit 1
fi

# shellcheck disable=SC2086
cc_build "$BIN_DIR/play-demo" tests/play_demo.c $ENGINE_SRC

case "$WAV" in
	1 | wav | WAV | true)
		mkdir -p "$BIN_DIR/wav"
		"$BIN_DIR/play-demo" "$DEMO" --wav "$BIN_DIR/wav/$DEMO.wav"
		;;
	*)
		"$BIN_DIR/play-demo" "$DEMO"
		;;
esac
