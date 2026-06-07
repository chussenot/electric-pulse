#!/bin/sh
# Build the MCP server (bin/electric-pulse-mcp).
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"
. scripts/lib.sh

MCP_BIN="$BIN_DIR/electric-pulse-mcp"
MCP_CFLAGS="-Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600 -Imcp/vendor/yyjson -Isrc"
# yyjson is third-party; suppress its warnings without diluting ours.
MCP_VENDOR_CFLAGS="-O2 -std=c99 -w -Imcp/vendor/yyjson"

mkdir -p "$BIN_DIR" build/mcp
# shellcheck disable=SC2086
$CC $MCP_VENDOR_CFLAGS -c mcp/vendor/yyjson/yyjson.c -o build/mcp/yyjson.o
# shellcheck disable=SC2086
$CC $MCP_CFLAGS -o "$MCP_BIN" tools/electric_pulse_mcp.c $ENGINE_SRC build/mcp/yyjson.o -lm
