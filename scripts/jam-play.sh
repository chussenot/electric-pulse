#!/bin/sh
# Jam a named demo and stream variations through aplay (Linux/ALSA).
# Usage: scripts/jam-play.sh <demo> [seed] [section-seconds]
#   or:  DEMO=<demo> SEED=42 SECTION=30 scripts/jam-play.sh
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

DEMO="${1:-${DEMO:-}}"
SEED="${2:-${SEED:-}}"
SECTION="${3:-${SECTION:-}}"

if [ -z "$DEMO" ]; then
	echo "Usage: mise run jam-play <demo> [seed] [section-seconds]" >&2
	exit 1
fi

scripts/jam.sh

set -- "data/music/$DEMO.abc"
[ -n "$SEED" ] && set -- "$@" --seed "$SEED"
[ -n "$SECTION" ] && set -- "$@" --section-seconds "$SECTION"

bin/electric-pulse-jam "$@" | aplay -q -f U8 -r 22050 -c 1
