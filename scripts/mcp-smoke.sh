#!/bin/sh
# Build the MCP server and pipe a few JSON-RPC requests through it.
set -eu
ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
cd "$ROOT"

scripts/mcp.sh

printf '%s\n%s\n%s\n' \
	'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
	'{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
	'{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"electric_pulse_engine_caps","arguments":{}}}' \
	| bin/electric-pulse-mcp
