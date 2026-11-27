PREFIX  ?= /usr/local
CC      ?= cc
CFLAGS  ?= -Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600
LDFLAGS ?= -lm

# The engine is Sonomorph's chip8, pinned as the vendor/sonomorph submodule
# (docs/adr/adr-0004-sonomorph-engine.md).
ENGINE_DIR = vendor/sonomorph/engine/chip8
ENGINE_SRC = $(ENGINE_DIR)/abc.c $(ENGINE_DIR)/audio_mix.c $(ENGINE_DIR)/audio_seq.c $(ENGINE_DIR)/audio_dsp.c $(ENGINE_DIR)/audio_fx.c $(ENGINE_DIR)/audio_song_builtin.c $(ENGINE_DIR)/audio_engine.c
ENGINE_HDR = $(wildcard $(ENGINE_DIR)/*.h)
ENGINE_CFLAGS = $(CFLAGS) -I$(ENGINE_DIR)

# An empty submodule: say how to fill it, unless only cleaning or reading help.
ifeq ($(wildcard $(ENGINE_DIR)/abc.c),)
ifneq ($(filter-out clean help,$(or $(MAKECMDGOALS),help)),)
$(error vendor/sonomorph is empty: run 'git submodule update --init')
endif
endif

BENCH        = bin/bench-audio
TEST_ABC_BIN = bin/test-abc
RENDER_DEMOS_BIN = bin/render-demos
PLAY_DEMO_BIN = bin/play-demo
EXPORT_MIDI_BIN = bin/export-midi
MCP_BIN = bin/electric-pulse-mcp
JAM_BIN = bin/electric-pulse-jam

.PHONY: clean test test-engine test-abc bench-audio render-demos play-demo export-midi gui-check gui-test gui-run mcp mcp-smoke jam jam-play help

.DEFAULT_GOAL := help

clean:
	rm -f $(BENCH) $(TEST_ABC_BIN) $(RENDER_DEMOS_BIN) $(PLAY_DEMO_BIN) $(EXPORT_MIDI_BIN) $(MCP_BIN) $(JAM_BIN)

test: test-engine test-abc
	@echo "All tests passed."

# The engine's own tests (DSP, sequencer, FX, render contracts) live with it
# in Sonomorph; this runs them at the pinned commit.
test-engine:
	cargo test --manifest-path vendor/sonomorph/Cargo.toml --features jam,midi

test-abc:
	@$(MAKE) --no-print-directory -B $(TEST_ABC_BIN)
	@echo "Running ABC parser tests..."
	@$(TEST_ABC_BIN)

$(TEST_ABC_BIN): tests/test_abc.c $(ENGINE_SRC) $(ENGINE_HDR)
	@mkdir -p bin
	$(CC) $(ENGINE_CFLAGS) -o $@ tests/test_abc.c $(ENGINE_SRC) $(LDFLAGS)

bench-audio: $(ENGINE_DIR)/audio_dsp.c tests/bench_audio.c
	@mkdir -p bin
	$(CC) $(ENGINE_CFLAGS) -o $(BENCH) tests/bench_audio.c $(ENGINE_DIR)/audio_dsp.c $(LDFLAGS)
	@echo "Build complete: $(BENCH)"
	@$(BENCH)

render-demos:
	@$(MAKE) --no-print-directory -B $(RENDER_DEMOS_BIN)
	@$(RENDER_DEMOS_BIN)

$(RENDER_DEMOS_BIN): tests/render_demos.c $(ENGINE_SRC) $(ENGINE_HDR)
	@mkdir -p bin
	$(CC) $(ENGINE_CFLAGS) -o $@ tests/render_demos.c $(ENGINE_SRC) $(LDFLAGS)

play-demo:
	@if [ -z "$(DEMO)" ]; then \
		echo "Usage: make play-demo DEMO=name [WAV=1]"; \
		echo "Examples:"; \
		echo "  make play-demo DEMO=dark_moroder"; \
		echo "  make play-demo DEMO=neon_nightdrive WAV=1"; \
		exit 1; \
	fi
	@$(MAKE) --no-print-directory -B $(PLAY_DEMO_BIN)
	@if [ "$(WAV)" = "1" ]; then \
		mkdir -p bin/wav; \
		$(PLAY_DEMO_BIN) "$(DEMO)" --wav "bin/wav/$(DEMO).wav"; \
	else \
		$(PLAY_DEMO_BIN) "$(DEMO)"; \
	fi

$(PLAY_DEMO_BIN): tests/play_demo.c $(ENGINE_SRC) $(ENGINE_HDR)
	@mkdir -p bin
	$(CC) $(ENGINE_CFLAGS) -o $@ tests/play_demo.c $(ENGINE_SRC) $(LDFLAGS)

# Read-only SeqSong -> MIDI exporter (ADR-0003). Off the render path: it links
# the parser/sequencer sources only to build a SeqSong, never the audio engine.
export-midi: $(EXPORT_MIDI_BIN)

$(EXPORT_MIDI_BIN): tests/export_midi.c $(ENGINE_DIR)/midi_export.c $(ENGINE_SRC) $(ENGINE_HDR)
	@mkdir -p bin
	$(CC) $(ENGINE_CFLAGS) -o $@ tests/export_midi.c $(ENGINE_DIR)/midi_export.c $(ENGINE_SRC) $(LDFLAGS)

gui-check:
	cargo check

gui-test:
	cargo test

gui-run:
	cargo run

MCP_ENGINE_SRC = $(ENGINE_SRC) $(ENGINE_DIR)/midi_export.c
MCP_CFLAGS = -Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600 -Imcp/vendor/yyjson -I$(ENGINE_DIR)
# yyjson is third-party; suppress its warnings without diluting ours.
MCP_VENDOR_CFLAGS = -O2 -std=c99 -w -Imcp/vendor/yyjson

mcp: $(MCP_BIN)

$(MCP_BIN): tools/electric_pulse_mcp.c $(MCP_ENGINE_SRC) $(ENGINE_HDR) mcp/vendor/yyjson/yyjson.c mcp/vendor/yyjson/yyjson.h
	@mkdir -p bin build/mcp
	$(CC) $(MCP_VENDOR_CFLAGS) -c mcp/vendor/yyjson/yyjson.c -o build/mcp/yyjson.o
	$(CC) $(MCP_CFLAGS) -o $@ tools/electric_pulse_mcp.c $(MCP_ENGINE_SRC) build/mcp/yyjson.o -lm

mcp-smoke: $(MCP_BIN)
	@printf '%s\n%s\n%s\n' \
	  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
	  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
	  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"electric_pulse_engine_caps","arguments":{}}}' \
	  | $(MCP_BIN)

JAM_ENGINE_SRC = $(ENGINE_SRC) $(ENGINE_DIR)/audio_jam.c
JAM_CFLAGS = -Wall -Wextra -O2 -std=c99 -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600 -I$(ENGINE_DIR)

jam: $(JAM_BIN)

$(JAM_BIN): tools/electric_pulse_jam.c $(JAM_ENGINE_SRC) $(ENGINE_HDR)
	@mkdir -p bin
	$(CC) $(JAM_CFLAGS) -o $@ tools/electric_pulse_jam.c $(JAM_ENGINE_SRC) -lm

# Convenience: jam a named demo and pipe to aplay (Linux/ALSA).
# Usage: make jam-play DEMO=three_chord_howl [SEED=42] [SECTION=30]
jam-play: $(JAM_BIN)
	@if [ -z "$(DEMO)" ]; then \
		echo "Usage: make jam-play DEMO=name [SEED=N] [SECTION=secs]"; \
		exit 1; \
	fi
	@$(JAM_BIN) "data/music/$(DEMO).abc" \
	  $(if $(SEED),--seed $(SEED)) \
	  $(if $(SECTION),--section-seconds $(SECTION)) \
	  | aplay -q -f U8 -r 22050 -c 1

help:
	@echo "Electric Pulse - Sound Machine (engine: vendor/sonomorph)"
	@echo ""
	@echo "Usage: make <target>"
	@echo ""
	@echo "Targets:"
	@echo "  clean      Remove compiled binaries"
	@echo "  test       Run the full test suite (the engine's own tests, ABC)"
	@echo "  test-engine Run the engine's own tests (Sonomorph, at the pinned commit)"
	@echo "  test-abc   Build and run ABC parser + showcase tests"
	@echo "  bench-audio Build and run audio microbenchmark"
	@echo "  render-demos Render showcase ABC demos and print deterministic metrics"
	@echo "  play-demo DEMO=name [WAV=1] Render a single demo and optionally export WAV"
	@echo "  export-midi Build the read-only SeqSong -> MIDI exporter (bin/export-midi)"
	@echo "  gui-check  Run cargo check for the GUI crate"
	@echo "  gui-test   Run cargo test for the GUI crate"
	@echo "  gui-run    Run the GUI crate"
	@echo "  mcp        Build the MCP server (bin/electric-pulse-mcp)"
	@echo "  mcp-smoke  Pipe a few JSON-RPC requests through the MCP server"
	@echo "  jam        Build the infinite-continuation player (bin/electric-pulse-jam)"
	@echo "  jam-play DEMO=name [SEED=N] [SECTION=secs]   Stream variations through aplay (Linux)"
	@echo "  help       Show this help message (default)"
