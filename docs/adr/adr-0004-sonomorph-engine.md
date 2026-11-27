---
id: ADR-0004
title: "The sound engine moves to Sonomorph; Electric Pulse consumes it"
status: Accepted
date: 2026-11-27
deciders:
  - "Electric Pulse maintainers"
authors:
  - "Claude Code"
supersedes: []
superseded_by: []
related_adrs:
  - "docs/adr-0001-gui-direct-audio-playback.md"
  - "docs/adr-0002-moog-ladder-filter.md"
  - "docs/adr/adr-0003-magenta-realtime-2-neural-audio-bridge.md"
decision_summary: >
  The C engine (parser, sequencer, mixer, DSP, FX, jam generator, MIDI
  exporter) lives in Sonomorph (github.com/chussenot/sonomorph, module
  chip8), pinned here as the vendor/sonomorph submodule. The Rust GUI reaches
  it through Sonomorph's safe API, never through a mirror of its C structs;
  the C tools compile its sources from the submodule. Electric Pulse adopts
  BRYX's fork of the engine, so every render golden was re-recorded once.
invariants_protected:
  - "one engine, one pin: the GUI, the tests and the C tools build the same commit"
  - "no Rust mirror of an engine struct's layout"
  - "showcase-goldens (tests/test_abc.c), re-recorded once, with lengths unchanged"
  - "render-path-consistency and offline-embeddable (ADR-0003)"
affects_files:
  - "vendor/sonomorph (new submodule), Cargo.toml (path dependency), build.rs (removed)"
  - "src/ffi.rs -> src/chip8.rs (adapter over sonomorph::chip8)"
  - "src/*.c, src/*.h (removed: now in vendor/sonomorph/engine/chip8)"
  - "tests/test_audio_dsp.c, tests/test_audio_seq.c (moved to Sonomorph)"
  - "tests/test_abc.c (goldens), Makefile, scripts/, .mise.toml, CI"
tags: [adr, engine, sonomorph, dependency, ffi, goldens, determinism]
agent_context: |
  RULE FOR AGENTS: engine changes (anything under vendor/sonomorph) are made in
  Sonomorph, by a pull request there, then pinned here by moving the
  submodule. Never edit vendor/sonomorph in place, never copy engine sources
  back into src/, and never declare a #[repr(C)] mirror of AbcMusic, SeqSong
  or any other engine struct: use sonomorph::chip8 (Song, render_file_u8,
  JamSession, export_midi). A golden in tests/test_abc.c changes only with a
  reason recorded next to it.
---

# ADR-0004: The sound engine moves to Sonomorph; Electric Pulse consumes it

## Context

Electric Pulse's C engine was copied into BRYX (a game) at commit `06c79fe`,
and forked there: 16-bit internal precision, held notes, dynamics, chords,
ramps, reverb, seamless loops, larger limits. The fork then moved into
Sonomorph, a library both games can depend on. That left two copies of one
engine, diverging, and Electric Pulse on the older one.

Electric Pulse also read the parser's `AbcMusic` through a `#[repr(C)]`
mirror in `src/ffi.rs`. That mirror is only correct while the C layout does
not change. BRYX's fork made `AbcMusic` nineteen times larger (69,472 bytes
to 1,338,736) by raising its limits. A mirror built against the old layout
would read the wrong bytes, and C code compiled against the old headers but
linked with the new engine overflows its buffers (seen under ASan while
evaluating this change).

## Options

1. **Keep the copy.** No work now; the copies keep diverging, and every fix
   is made twice or lost.
2. **Sonomorph as a Cargo git dependency.** One pin for the Rust GUI. The C
   tools (MCP server, jam player, demo renderer, tests) would need the
   engine sources from Cargo's checkout, found through `cargo metadata` and
   a JSON parser in every script.
3. **Sonomorph as a submodule, used by Cargo as a path dependency.** One
   pin, `vendor/sonomorph`, read by Cargo and by `make`/`scripts/` alike.
   Every clone needs `git submodule update --init`.

## Decision

Option 3. Sonomorph grew the API the GUI needs so that nothing here touches
an engine struct:

| Electric Pulse used | Sonomorph provides |
|---|---|
| `audio_engine_render_abc_file` + `AudioRenderStats` | `render_file_u8` + `RenderStats` |
| `abc_load` into a mirrored `AbcMusic` | `Song::load`, read through view structs Sonomorph owns, with a size test on both sides |
| `audio_jam_session_*` | `JamSession` (feature `jam`) |
| `midi_export_abc_file` | `export_midi` (feature `midi`) |

`src/chip8.rs` (formerly `src/ffi.rs`) adapts these to the GUI's overview
and editor structs, defaults and `String` errors. It holds no `unsafe`.
Each render now returns its own statistics, so the global "last render
stats" slot (which two threads rendering at once could cross) is gone.

The engine's own C tests (`test_audio_dsp.c`, `test_audio_seq.c`) moved
with it; `make test-engine` runs Sonomorph's suite at the pinned commit.
`test_abc.c` stays here: it checks Electric Pulse's songs.

## Consequences

**The sound changed once, on purpose.** The fork mixes at 16 bits and
dithers once to 8, where the original rounded at every voice and multiply.
The maintainers listened and approved the result. Measured on the 16
showcase demos:

- every demo renders to exactly the same length (timeline and arrangement
  unchanged);
- the golden fixture's first 32 samples moved by at most one step;
- clipping is of the same order as before, higher on some demos and lower
  on others (Metro Chase 10,420 to 8,010; Carpenter Drive 3,803 to 4,640).
  It comes from how hot those demos are mixed, not from the engine;
- every checksum in `tests/test_abc.c` was re-recorded, once, with this ADR
  as the reason.

**Larger limits.** The engine now accepts 12 voices, 4,096 steps, 12
instruments, 64 patterns and 64 arrangement slots (the MCP server's
`electric_pulse_engine_caps` reports them). The GUI still shows 8 lanes;
the rest are counted in `hidden_track_count`, as before. The editor's own
validation is unchanged.

**Stack use in the C tools.** `AbcMusic` and `SeqSong` are now about 1.3 MB
and 0.65 MB. The tools keep them on the main thread's stack (8 MB on Linux
and macOS); the jam player peaks near 2.6 MB. A tool that moves this work to
another thread must allocate them on the heap, as Sonomorph's jam and MIDI
code now does.

**Determinism.** The FX layer (`audio_fx.c`) is integer (Q15) in both
engines. Both use floating point for pitch and oscillator phase; the fork
adds a few doubles (the single dither step, some DSP setup). The goldens
render identically at `-O0` and `-O2`, and Sonomorph's probes identically
on developer machines and CI.

**Workflow.** A clone needs `git clone --recursive` or
`git submodule update --init`; `make` stops with that instruction when the
submodule is empty. An engine change is a Sonomorph pull request, then a
submodule bump here.

## Validation

- `cargo test`: the GUI's 46 tests, including jam sessions, MIDI export and
  the overview of an invalid score, pass on Sonomorph.
- `make test`: Sonomorph's suite at the pin, and `test_abc.c` with the new
  goldens; `test_abc.c` is also clean under ASan and UBSan.
- `make bench-audio`, `make mcp-smoke`, `make jam`, `make export-midi`,
  `make render-demos`, `make play-demo DEMO=dark_moroder WAV=1` all build
  and run against the submodule.
- The headless GUI smoke run (CI's `gui` job) boots, renders and plays a
  demo.
