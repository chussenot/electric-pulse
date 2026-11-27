# Electric Pulse GUI Engine and Playback Boundary

## Engine boundary

The engine is Sonomorph's `chip8` module, pinned at `vendor/sonomorph`
([ADR-0004](adr/adr-0004-sonomorph-engine.md)). Sonomorph owns every
`unsafe` call into the C engine; Electric Pulse has none.

`src/chip8.rs` is the only module that calls Sonomorph:

| Call | Used for |
|---|---|
| `chip8::render_file_u8(path, 22_050)` | Rendering a demo or the editor's preview: U8 PCM and its `RenderStats` |
| `chip8::Song::load(path)` | The browser's overview and the editor's song, before any render |
| `chip8::JamSession` | Jam mode, on its worker thread |
| `chip8::export_midi(abc, out)` | MIDI export (off the render path, ADR-0003) |

No UI code calls Sonomorph directly.

## Metadata extraction

`Song::load` returns owned Rust values: title, tempo, swing, instruments,
patterns, arrangement, FX buses, and each voice with its frequency on every
step. `src/chip8.rs` converts a `Song` into:

- `DemoOverview`
- `TrackOverview`
- `FxBusOverview`
- `PatternBlock`
- `StepState`

and, for the editor, into `RawAbcMusicForEditor`. This is where waveform
names, inferred beat markers, arrangement blocks, the GUI's eight visible
lanes, and bus defaults are normalized for the GUI.

## Memory ownership

Sonomorph copies the engine's PCM into a `Vec<u8>` and frees the C
allocation before returning, so the GUI only ever holds Rust-owned memory.
`render_abc_file` returns the samples together with their `RenderStats`;
there is no shared "last render" state.

## Playback boundary

`src/playback.rs` is intentionally outside the engine boundary.

- rendered PCM is copied into Rust-owned memory and streamed directly with `cpal`
- playback progress is tracked from consumed sample cursor over full render length
- stop/poll always clean up stream ownership and playback runtime state

## Limitations

- playback depends on host output device availability and a valid default stream config
- no separate low-latency GUI mixer is introduced; playback is output-stream only
- metadata is for inspection only; the GUI never edits engine structures
