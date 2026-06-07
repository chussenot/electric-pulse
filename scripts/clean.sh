#!/bin/sh
# Remove compiled C binaries.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

rm -f \
	bin/bench-audio \
	bin/test-audio-dsp \
	bin/test-abc \
	bin/test-audio-seq \
	bin/render-demos \
	bin/play-demo \
	bin/electric-pulse-mcp \
	bin/electric-pulse-jam
