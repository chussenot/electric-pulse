---
title: "Leveraging Google Magenta RealTime 2 in Electric Pulse"
type: study
status: complete
date: 2026-06-13
authors:
  - "Claude Code (Opus 4.8)"
reviewers:
  - "Electric Pulse maintainers"
methodology: "Orchestra Research — AI Research Skills (autoresearch Bootstrap phase + Rigor Reviewer lens)"
skills_applied:
  - "0-autoresearch-skill (literature survey, gap identification, hypothesis framing)"
  - "11-evaluation (fit-against-invariants assessment)"
  - "18-multimodal (AudioCraft-class audio-generation framing)"
  - "22-agent-native-research-artifact / Rigor Reviewer (decision provenance, severity-ranked risks)"
skills_source: "https://www.orchestra-research.com/ai-research-skills/welcome.md"
skills_repo: "https://github.com/Orchestra-Research/AI-Research-SKILLs"
subject_model:
  id: "google/magenta-realtime-2"
  hf_url: "https://huggingface.co/google/magenta-realtime-2"
  task: "text-to-audio (music generation, real-time streaming)"
  license: "Apache-2.0 (code) / CC-BY-4.0 (weights)"
demo_spaces:
  - "https://huggingface.co/spaces/magenta-community/magenta-rt-studio"
  - "https://huggingface.co/spaces/magenta-community/magenta-rt-jam"
  - "https://huggingface.co/spaces/magenta-community/magenta-rt-collider"
references:
  - "arXiv:2508.04651 — Live Music Models (Lyria Team) — covers Magenta RealTime AND Lyria RealTime"
  - "arXiv:2508.05207 — Magenta RealTime / codec supporting reference"
  - "arXiv:2205.01917 — CoCa: Contrastive Captioners (basis for MusicCoCa)"
  - "arXiv:2208.12415 — referenced audio-modelling work"
related_adr: "docs/adr/adr-0003-magenta-realtime-2-neural-audio-bridge.md"
related_docs:
  - "docs/audio-architecture.md"
  - "docs/adr-0001-gui-direct-audio-playback.md"
  - "docs/adr-0002-moog-ladder-filter.md"
  - "CLAUDE.md (render-path-consistency invariant)"
project_invariants_touched:
  - "render-path-consistency (single C entry: audio_engine_render_abc_file)"
  - "q15-fixed-point-determinism (no floats in render path)"
  - "showcase-goldens (bit-exact FNV-1a-64 PCM checksums in tests/test_abc.c)"
  - "offline-embeddable (no network / no GPU at render time)"
recommendation: "Adopt as an adjacent, OPTIONAL neural-audio companion via a read-only MIDI export bridge. Never inside the render path. (See ADR-0003.)"
tags: [magenta, lyria, neural-audio, midi, music-generation, feasibility, study, ffi, jam]
agent_context: |
  Feasibility study for whether/how Electric Pulse should use Google's
  Magenta RealTime 2 model. KEY FACT: the model consumes MIDI as a *control
  input* and emits 48 kHz stereo *audio* — it is a neural synth/continuation
  engine, not a symbolic/MIDI generator. It is the architectural OPPOSITE of
  Electric Pulse's deterministic Q15 C engine, so it cannot live inside the
  render path. The viable leverage is an adjacent, opt-in bridge. Read this
  before proposing any Magenta/neural-audio integration; the decision is
  recorded in ADR-0003.
---

# Study: Leveraging Google Magenta RealTime 2 in Electric Pulse

> Applied methodology: Orchestra Research **AI Research Skills**, `autoresearch`
> *Bootstrap* phase (literature survey → gap identification → hypothesis framing),
> with a *Rigor Reviewer* pass for severity-ranked risk. This is a feasibility /
> model-leverage study, not a training experiment, so the inner/outer experiment
> loops are framed as proposed next steps rather than executed runs.

## 1. Research question

Can Electric Pulse — a deterministic, integer fixed-point C music engine — gain
value from Google's **Magenta RealTime 2** (`google/magenta-realtime-2`), and if
so, *where in the architecture* does it belong without breaking the project's
load-bearing invariants?

## 2. Bootstrap — literature & source survey

### 2.1 What Magenta RealTime 2 is

An **open-weights, real-time music generation model** from Google, built for
on-device streaming generation with low-latency control. It is the open sibling
of **Lyria RealTime** (the API-only DeepMind model); both are described in the
same paper, *Live Music Models* (arXiv:2508.04651, Lyria Team). This is the
direct answer to the earlier "find the latest Google model for music/MIDI"
question: Lyria itself is closed/API-gated and audio-only, but Magenta RealTime 2
is its downloadable counterpart.

| Property | Value |
| --- | --- |
| Task | text-to-audio (music), streaming |
| **Inputs** | text style prompt; audio style example (→ MusicCoCa embedding); **MIDI control** (128-dim multi-hot pitch-state vectors, onset/sustain distinction, frame-by-frame); context audio tokens |
| **Outputs** | **48 kHz stereo audio waveforms** via the SpectroStream codec (discrete audio tokens, 25 Hz frame rate, 64-deep RVQ, 10-bit codes) |
| Architecture | SpectroStream codec + MusicCoCa (768-dim joint text-audio embedding) + decoder-only Transformer LLM |
| Sizes | Base 2.4B params (20 layers) / Small 230M params (12 layers); ~20 s effective receptive field |
| Latency | ~200 ms streaming; 25 Hz autoregressive frames (40 ms/frame) |
| Runtime | JAX + Sequence Layers; trained on TPU; Colab/Kaggle; community ONNX/CoreML/TFLite on-device ports exist |
| License | Apache-2.0 (code), CC-BY-4.0 (weights) |
| Training data | ~71k h predominantly **instrumental** music; genre-coverage gaps acknowledged |

The `magenta-rt-studio` Space (Gradio) is the reference interactive front-end:
text/audio prompting, real-time knob-style steering, continuous generation. The
`magenta-rt-jam` Space demonstrates the live-jam loop. These are the closest
existing UX analog to Electric Pulse's own **jam** feature.

### 2.2 The decisive finding (the gap)

Magenta RT 2 is **MIDI-in / audio-out**. It does *not* generate MIDI or symbolic
note data — it generates a neural waveform, optionally *steered* by MIDI. That
single fact reframes the whole question:

- It is **not** a drop-in for anything Electric Pulse currently does symbolically
  (ABC parsing, `SeqSong` step sequencing, deterministic mixing).
- It **is** a candidate neural *synthesizer / continuation* engine that could be
  *fed by* Electric Pulse's existing note data.

### 2.3 Related controllable-music work (for context)

- **Music ControlNet** (arXiv:2311.07069) and **JASCO** (arXiv:2406.10970) —
  symbolic+audio time-varying conditioning of diffusion music models. Confirms
  "symbolic control → neural audio" is an established pattern, not a one-off.
- **MusicCoCa** derives from **CoCa** (arXiv:2205.01917).

## 3. Fit assessment against Electric Pulse invariants

The Rigor-Reviewer lens scores each invariant by how a *render-path* integration
would impact it. Severity: 🟥 fatal · 🟧 high · 🟨 moderate · 🟩 fine.

| Invariant (see `CLAUDE.md`, ADR-0001/0002) | Electric Pulse today | Magenta RT 2 | In-render-path impact |
| --- | --- | --- | --- |
| **Render-path consistency** — one C entry `audio_engine_render_abc_file` | All consumers (CLI, tests, GUI, MCP) share it | A 230M–2.4B JAX model | 🟥 cannot be the shared path |
| **Determinism** — Q15 integer, no floats, reproducible | Bit-exact across platforms | Sampling-based, float, non-deterministic | 🟥 breaks reproducibility |
| **Showcase goldens** — bit-exact FNV-1a-64 PCM checksums (`tests/test_abc.c`) | Locked for 10 demos | Output differs every run | 🟥 ungoldennable |
| **Output format** — u8 PCM, 22050 Hz, mono | Tiny, embeddable | 48 kHz stereo float | 🟧 format + scale mismatch |
| **Offline / embeddable** — no network, no GPU | Pure C, runs anywhere | TPU/GPU or heavy on-device runtime | 🟧 deployment weight |
| **Dependency surface** — C99, no language mix preferred | Minimal | JAX/Python or large ONNX/CoreML artifact | 🟧 new toolchain |

**Conclusion of the assessment:** Magenta RT 2 is the architectural *opposite* of
the engine. Any attempt to put it *inside* `audio_engine_render_abc_file` is fatal
to determinism and the regression goldens (three 🟥 rows). The only sound place
for it is **adjacent and optional**, reached through a clean boundary, and
explicitly excluded from the showcase regression contract.

## 4. Leverage options (scored)

Scores 1 (worst) – 5 (best), in the house ADR style.

| Option | Invariant safety | Musical value | Effort | Dep/runtime cost | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| **A. Reference only** (document, no code) | 5 | 1 | 5 | 5 | 16 |
| **B. MIDI export bridge** (read-only `SeqSong`→MIDI; user feeds the HF Space / Colab) | 5 | 4 | 4 | 5 | 18 |
| **C. Neural-jam companion** (opt-in GUI mode calling Magenta RT via local server / HF Inference API, fenced off from the engine) | 3 | 5 | 2 | 2 | 12 |
| **D. Neural synthesis in the render path** (replace/augment the C mixer) | 1 | 5 | 1 | 1 | 8 |

### Why the scores

- **B (18) — recommended first step.** A read-only exporter that walks `SeqSong`
  steps (pitch, onset, velocity already exist) and emits a standard `.mid` file
  *never touches the render path*, adds *zero* runtime deps, and stays
  deterministic. It immediately unlocks the model: the user drops the MIDI into
  `magenta-rt-studio` (or a local Colab) with a text style prompt and gets a
  neural-audio "skin" of an Electric Pulse track. Highest value-per-risk.
- **C (12) — viable later, opt-in only.** Electric Pulse already has a jam loop
  (`audio_jam.{c,h}`, Ctrl+J in the GUI, worker-thread continuation). A *neural*
  jam mode that streams Magenta RT 2 output alongside the deterministic jam is
  musically compelling and conceptually aligned — but it introduces
  non-determinism, a network/GPU dependency, and 48 kHz stereo float playback.
  Acceptable **only** behind a feature gate, with its audio path kept entirely
  separate from `audio_engine_render_abc_file` and out of `tests/test_abc.c`.
- **D (8) — rejected.** Shatters every invariant in §3. Out of scope.
- **A (16) — the floor.** Safe but leaves the value on the table; B dominates it.

## 5. Recommendation

Adopt **Option B now**: a read-only **MIDI export bridge** from `SeqSong`, with
Magenta RT 2 run *externally* (the `magenta-rt-studio` Space, Colab, or a
community on-device port). Treat **Option C** as a future, explicitly opt-in
experimental GUI feature. **Never Option D.** This keeps the deterministic C
engine — and the showcase goldens — completely untouched while giving composers a
neural-audio rendering path for their Electric Pulse material.

The decision and its boundary are recorded in
[`ADR-0003`](../adr/adr-0003-magenta-realtime-2-neural-audio-bridge.md).

## 6. Proposed experiments (autoresearch inner-loop hypotheses)

Pre-registered, smallest-first. Each is a locked prediction to confirm/refute.

1. **H1 — MIDI fidelity.** A `SeqSong`→MIDI exporter reproduces a demo's pitch
   and rhythm such that a DAW re-render is recognizably the same piece.
   *Predict:* pitch/onset map cleanly; the only loss is timbre + FX (expected,
   since FX is engine-side). *Artifact:* `data/music/*.mid` from `glass_anthem`.
2. **H2 — Steerability.** Feeding that MIDI + a genre text prompt to
   `magenta-rt-studio` yields audio that follows the score's harmony/rhythm
   rather than drifting. *Predict:* strong adherence on dense patterns, looser on
   sparse ones (per Music ControlNet behaviour).
3. **H3 — Neural-jam latency budget.** Measure end-to-end latency of a local /
   API Magenta RT call against the existing ~29 s pre-render gap-killer window in
   the jam worker. *Predict:* API round-trip blows the real-time budget; only a
   local on-device port (ONNX/TFLite/CoreML) is viable for live jam (informs C).
4. **H4 — Determinism fence.** Confirm that adding the exporter leaves every
   showcase golden in `tests/test_abc.c` bit-identical (`mise run test`).
   *Predict:* green with no golden refresh (exporter is read-only, off the render
   path).

## 7. Open questions / lessons & constraints

- **MIDI export is a prerequisite** and Electric Pulse has none today. It is the
  enabling, zero-risk unit of work — and useful on its own (DAW interop).
- **Licensing:** CC-BY-4.0 weights require attribution on any distributed neural
  audio; Apache-2.0 covers the code. Document attribution if Option C ships.
- **No bundling at render time:** the model must remain an *external* tool or an
  *opt-in* runtime, never a build/test dependency of the C engine or CI.
- **Lyria note:** if a hosted/managed path is ever preferred over local weights,
  Lyria RealTime (Gemini API / Vertex AI) is the same model family — same bridge
  (MIDI + text prompt), different transport.

## 8. Provenance

- Sources: HF model card `google/magenta-realtime-2`; Space
  `magenta-community/magenta-rt-studio`; arXiv:2508.04651; HF paper search.
- Methodology: AI Research Skills (`autoresearch` Bootstrap), installed per
  `https://www.orchestra-research.com/ai-research-skills/welcome.md`
  (`npx @orchestra-research/ai-research-skills install --all`).
- Decision record: `docs/adr/adr-0003-magenta-realtime-2-neural-audio-bridge.md`.
