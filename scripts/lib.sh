# shellcheck shell=sh
# Shared build configuration for Electric Pulse C targets.
# POSIX sh. Source this from task scripts; do not execute directly.

# Toolchain — overridable from the environment, same defaults as the old Makefile.
: "${CC:=cc}"
: "${CFLAGS:=-Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600}"
: "${LDFLAGS:=-lm}"

BIN_DIR="bin"

# Engine sources shared by every full-engine consumer (tests, demos, MCP, jam).
ENGINE_SRC="src/abc.c src/audio_mix.c src/audio_seq.c src/audio_dsp.c src/audio_fx.c src/audio_song_builtin.c src/audio_engine.c"

# Compile a C target. Usage: cc_build <output> <source...>
# CFLAGS / LDFLAGS are word-split on purpose so callers can pass multiple flags.
cc_build() {
	out="$1"
	shift
	mkdir -p "$(dirname "$out")"
	# shellcheck disable=SC2086
	$CC $CFLAGS -o "$out" "$@" $LDFLAGS
}
