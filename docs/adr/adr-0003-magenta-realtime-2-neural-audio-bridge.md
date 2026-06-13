---
id: ADR-0003
title: "Magenta RealTime 2 as an adjacent neural-audio bridge, not a render-path engine"
status: Accepted
date: 2026-06-13
deciders:
  - "Electric Pulse maintainers"
authors:
  - "Claude Code (Opus 4.8)"
supersedes: []
superseded_by: []
related_adrs:
  - "docs/adr-0001-gui-direct-audio-playback.md"
  - "docs/adr-0002-moog-ladder-filter.md"
related_study: "docs/studies/2026-06-13-magenta-realtime-2-leverage-study.md"
subject_model:
  id: "google/magenta-realtime-2"
  hf_url: "https://huggingface.co/google/magenta-realtime-2"
  demo_space: "https://huggingface.co/spaces/magenta-community/magenta-rt-studio"
  license: "Apache-2.0 (code) / CC-BY-4.0 (weights)"
decision_summary: >
  Adopt Magenta RealTime 2 only as an OPTIONAL, adjacent neural-audio companion
  reached through a read-only SeqSong→MIDI export bridge. It must never live
  inside audio_engine_render_abc_file, must add no build/CI dependency to the C
  engine, and must be excluded from the showcase regression goldens.
invariants_protected:
  - "render-path-consistency"
  - "q15-fixed-point-determinism"
  - "showcase-goldens (tests/test_abc.c)"
  - "offline-embeddable"
affects_files_if_implemented:
  - "src/audio_seq.h (read-only MIDI export helper signature)"
  - "new: src/midi_export.{c,h} (SeqSong → .mid, off the render path)"
  - "gui/ (opt-in neural-jam mode, feature-gated) — future, Option C only"
tags: [adr, magenta, lyria, neural-audio, midi, determinism, boundary, jam]
agent_context: |
  Decision record gating any Magenta/Lyria/neural-audio work in Electric Pulse.
  RULE FOR AGENTS: neural audio generation is allowed ONLY adjacent to the engine
  (export MIDI out; run the model externally or behind an opt-in GUI gate). Do NOT
  call a neural model from audio_engine_render_abc_file, do NOT add JAX/ONNX/Python
  to the C build or CI, and do NOT let neural output into tests/test_abc.c goldens.
  The enabling unit of work is a read-only SeqSong→MIDI exporter. Rationale and
  scoring live in the related study.
---

# ADR-0003: Magenta RealTime 2 as an adjacent neural-audio bridge, not a render-path engine

- Status: Accepted
- Date: 2026-06-13
- Deciders: Electric Pulse maintainers
- Related study: [Leveraging Magenta RealTime 2](../studies/2026-06-13-magenta-realtime-2-leverage-study.md)

## Context

Google released **Magenta RealTime 2** (`google/magenta-realtime-2`), an
open-weights real-time music model — the downloadable sibling of **Lyria
RealTime** (both from the *Live Music Models* paper, arXiv:2508.04651). It is
**MIDI-in / audio-out**: it accepts text prompts, an audio style example, and
frame-wise **MIDI control** (128-dim onset/sustain pitch vectors), and emits
**48 kHz stereo audio** through the SpectroStream codec. Sizes are 230M (Small)
and 2.4B (Base); runtime is JAX/TPU with community ONNX/CoreML/TFLite on-device
ports. The `magenta-rt-studio` Gradio Space is the reference UI, and
`magenta-rt-jam` mirrors a live-jam loop conceptually close to our own jam
feature.

The question is whether Electric Pulse should integrate it, and where. The full
fit assessment is in the related study; the short version: Magenta RT 2 is the
architectural *opposite* of this engine. Electric Pulse is deterministic,
integer **Q15** fixed-point, **no floats in the render path**, outputs **u8 PCM
@ 22050 Hz mono**, runs fully offline, and routes **every** consumer (CLI,
`mise run test`, GUI, MCP) through the single C entry point
`audio_engine_render_abc_file`. The showcase regression suite (`tests/test_abc.c`)
locks **bit-exact FNV-1a-64 PCM checksums** for ten demos.

A sampling-based neural model cannot satisfy any of those properties.

## Options scored

Scores: 1 (worst) to 5 (best). Detail and per-option rationale in the study (§4).

| Option | Invariant safety | Musical value | Effort | Dep/runtime cost | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| A. Reference only (no code) | 5 | 1 | 5 | 5 | 16 |
| **B. MIDI export bridge** (read-only `SeqSong`→`.mid`; model runs externally) | 5 | 4 | 4 | 5 | **18** |
| C. Neural-jam companion (opt-in, feature-gated GUI mode) | 3 | 5 | 2 | 2 | 12 |
| D. Neural synthesis inside the render path | 1 | 5 | 1 | 1 | 8 |

## Decision

Adopt **Option B now**, with **Option C** reserved as a future opt-in experiment
and **Option D rejected outright**:

1. Add a **read-only `SeqSong` → MIDI exporter** (`src/midi_export.{c,h}`,
   exposed via FFI and an MCP tool / CLI). It walks existing step data (pitch,
   onset, velocity) and writes a standard `.mid`. It is **off the render path** —
   it neither calls nor alters `audio_engine_render_abc_file`.
2. Magenta RealTime 2 runs **externally**: the user feeds the exported MIDI plus
   a text style prompt to `magenta-rt-studio`, a Colab/Kaggle notebook, or a
   community on-device port, and gets back a neural-audio rendering of the track.
3. A future **neural-jam GUI mode** (Option C) may stream Magenta RT 2 alongside
   the deterministic jam, but **only** behind a feature gate, with its audio path
   kept entirely separate from the C engine and **excluded** from the showcase
   goldens.

The model **must never** be called from `audio_engine_render_abc_file`, **must
never** become a build or CI dependency of the C engine, and its output **must
never** enter `tests/test_abc.c`.

## Rationale

- **Determinism is the product.** Three invariants are 🟥-fatal to an in-path
  integration: render-path consistency, Q15 determinism, and the bit-exact
  goldens. Option D violates all three; B violates none.
- **Highest value-per-risk.** The bridge adds zero runtime dependencies, stays
  deterministic, and immediately unlocks the model. MIDI export is independently
  useful (DAW interop) and is the prerequisite for every richer option.
- **Consistent with ADR-0001/0002.** ADR-0001 keeps playback (`cpal`) outside the
  C engine; ADR-0002 keeps DSP *inside* it as pure C. ADR-0003 extends the same
  boundary discipline: neural generation is an *adjacent* capability, never a
  substitute for the deterministic core.
- **Lyria continuity.** If a managed transport is later preferred, Lyria RealTime
  (Gemini/Vertex API) is the same model family and reuses the same MIDI+prompt
  bridge — only the transport changes.

## Consequences

Positive:

- The deterministic C engine and all showcase goldens are completely untouched;
  `mise run test` stays green with no golden refresh.
- Composers gain a neural-audio rendering path for Electric Pulse material with
  no new engine dependency.
- A standard `.mid` exporter is a reusable interop win on its own.

Trade-offs / constraints:

- Neural rendering is **not reproducible** and lives outside the project's
  guarantees by design; this must be clearly communicated wherever it surfaces.
- Option C, if pursued, carries non-determinism, a network/GPU dependency, and
  48 kHz stereo float playback — all of which must be fenced behind a feature
  gate and kept out of CI/goldens.
- CC-BY-4.0 weights require **attribution** on any distributed neural audio.

## Implementation notes

1. `src/midi_export.{c,h}`: `int midi_export_seq_song(const SeqSong *song, const
   char *path)` — pure read of timeline/track/step data → Standard MIDI File
   (format 1). No engine state mutated; no float math required.
2. Surface it without touching the render path: an MCP tool (`export_midi`) and a
   `mise run export-midi <name>` wrapper; FFI mirror in `src/ffi.rs` for the GUI.
3. **No-op for goldens:** because the exporter never feeds back into rendering,
   adding it must leave every `tests/test_abc.c` checksum bit-identical. Verify
   with `mise run test` before/after.
4. Option C scaffolding (deferred): isolate any Magenta call in a separate Rust
   module with its own playback buffer; gate behind a cargo feature
   (e.g. `neural-jam`) that is **off** in default builds and CI.

## Validation

- `mise run test` passes with no golden refresh after the exporter lands
  (exporter is read-only and off the render path).
- Round-trip spot check: export `glass_anthem` → `.mid`, import into a DAW or
  `magenta-rt-studio`, confirm pitch/rhythm match (study H1/H2).
- Confirm the C engine builds with no new dependencies; the model is invoked only
  externally or behind the (default-off) `neural-jam` gate.
