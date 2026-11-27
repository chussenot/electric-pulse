# shellcheck shell=sh
# Shared build configuration for Electric Pulse C targets.
# POSIX sh. Source this from task scripts; do not execute directly.

# Toolchain — overridable from the environment, same defaults as the old Makefile.
: "${CC:=cc}"
: "${CFLAGS:=-Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600}"
: "${LDFLAGS:=-lm}"

BIN_DIR="bin"

# The engine is Sonomorph's chip8, pinned as the vendor/sonomorph submodule
# (docs/adr/adr-0004-sonomorph-engine.md). Its headers and sources live here.
ENGINE_DIR="vendor/sonomorph/engine/chip8"

# Engine sources shared by every full-engine consumer (tests, demos, MCP, jam).
ENGINE_SRC="$ENGINE_DIR/abc.c $ENGINE_DIR/audio_mix.c $ENGINE_DIR/audio_seq.c $ENGINE_DIR/audio_dsp.c $ENGINE_DIR/audio_fx.c $ENGINE_DIR/audio_song_builtin.c $ENGINE_DIR/audio_engine.c"

# Stop with the fix when the submodule was never checked out.
require_engine() {
	if [ ! -f "$ENGINE_DIR/abc.c" ]; then
		echo "vendor/sonomorph is empty: run 'git submodule update --init'" >&2
		exit 1
	fi
}

# Compile a C target against the engine's headers.
# Usage: cc_build <output> <source...>
# CFLAGS / LDFLAGS are word-split on purpose so callers can pass multiple flags.
cc_build() {
	require_engine
	out="$1"
	shift
	mkdir -p "$(dirname "$out")"
	# shellcheck disable=SC2086
	$CC $CFLAGS -I"$ENGINE_DIR" -o "$out" "$@" $LDFLAGS
}
