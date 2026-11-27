//! Electric Pulse's side of the sound engine.
//!
//! The engine is Sonomorph's `chip8` (`vendor/sonomorph`, see
//! `docs/adr/adr-0004-sonomorph-engine.md`): the C renderer, parser, jam
//! generator and MIDI exporter that used to live in `src/*.c`. Sonomorph
//! owns the unsafe boundary; this module turns what it returns into what
//! the GUI shows and edits, with the GUI's own defaults and limits, and
//! keeps the GUI's `String` errors.

use std::collections::HashMap;
use std::path::Path;

use sonomorph::chip8::song::{FxBus, Waveform};
use sonomorph::chip8::{self, Song};

pub use sonomorph::chip8::{JamSection, MidiExportStats, RenderStats as AudioRenderStats};

use crate::audio_engine::{DemoOverview, FxBusOverview, PatternBlock, StepState, TrackOverview};

/// Every render, preview and jam section plays at this rate.
pub const SAMPLE_RATE_ABC: u32 = 22_050;
/// What the inspector shows when a voice leaves vibrato rate at the
/// engine's default.
const ABC_DEFAULT_VIBRATO_RATE: i32 = 5_200;
/// What the inspector shows when a bus leaves sidechain release at the
/// engine's default.
const ABC_DEFAULT_SIDECHAIN_RELEASE_MS: i32 = 180;
/// Tracks the browser's lanes show; the rest are counted as hidden. The
/// engine plays more (Sonomorph's limits are larger than the GUI's layout).
const GUI_VISIBLE_TRACKS: usize = 8;

/// The GUI's own wording for a path that is not there, before the engine
/// is asked anything.
fn checked_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("missing demo file: {}", path.display()));
    }
    if !path.is_file() {
        return Err(format!("demo path is not a file: {}", path.display()));
    }
    Ok(())
}

/// Renders a score as unsigned 8-bit mono at [`SAMPLE_RATE_ABC`], with
/// its statistics.
pub fn render_abc_file(path: &Path) -> Result<(Vec<u8>, AudioRenderStats), String> {
    checked_path(path)?;
    chip8::render_file_u8(path, SAMPLE_RATE_ABC).map_err(|err| format!("render failure: {err}"))
}

/// Exports a score to a Standard MIDI File (ADR-0003: off the render path,
/// so it never touches a render or the showcase goldens). `out_path`'s
/// directory must exist; a failed export leaves `out_path` as it was.
pub fn export_abc_to_midi(abc_path: &Path, out_path: &Path) -> Result<MidiExportStats, String> {
    checked_path(abc_path)?;
    chip8::export_midi(abc_path, out_path).map_err(|err| {
        format!(
            "MIDI export failed: {err} ({} -> {})",
            abc_path.display(),
            out_path.display()
        )
    })
}

/// A score played on without end, section by section: each section a
/// seeded variation of the arrangement. Moves to the jam worker thread.
pub struct JamSession(chip8::JamSession);

impl JamSession {
    pub fn open(path: &Path, seed: u64, section_seconds: f64) -> Result<Self, String> {
        checked_path(path)?;
        chip8::JamSession::open(path, seed, section_seconds)
            .map(Self)
            .map_err(|err| format!("could not open jam session: {err}"))
    }

    pub fn render_next(&mut self) -> Result<JamSection, String> {
        self.0
            .render_next(SAMPLE_RATE_ABC)
            .map_err(|err| format!("jam render failed: {err}"))
    }
}

fn load_song(path: &Path) -> Result<Song, String> {
    checked_path(path)?;
    Song::load(path).map_err(|err| format!("invalid ABC or unreadable demo: {err}"))
}

/// What the browser shows of a demo: its lanes, arrangement and buses.
pub fn load_demo_overview(path: &Path) -> Result<DemoOverview, String> {
    build_demo_overview(&load_song(path)?)
}

fn build_demo_overview(song: &Song) -> Result<DemoOverview, String> {
    let preset_map = build_instrument_preset_map(song);
    let visible_tracks = song.voices.len().min(GUI_VISIBLE_TRACKS);
    let hidden_track_count = song.voices.len() - visible_tracks;
    let visible = &song.voices[..visible_tracks];
    let max_steps = visible
        .iter()
        .map(|voice| voice.steps.len())
        .max()
        .unwrap_or(0);
    let steps_per_beat = inferred_steps_per_beat(song);

    let arrangement = build_arrangement(song, max_steps)?;
    let total_steps = arrangement
        .last()
        .map(|block| block.start_step + block.length)
        .unwrap_or(max_steps);

    let tracks = visible
        .iter()
        .enumerate()
        .map(|(index, voice)| {
            build_track_overview(index, voice, &preset_map, total_steps, steps_per_beat)
        })
        .collect();

    Ok(DemoOverview {
        title: song.title.clone(),
        bpm: song.bpm,
        swing_pct: if song.swing_pct <= 0 {
            50
        } else {
            song.swing_pct
        },
        steps_per_beat,
        total_steps: total_steps.max(max_steps),
        arrangement,
        tracks,
        fx_buses: build_fx_buses(song),
        hidden_track_count,
    })
}

fn build_track_overview(
    index: usize,
    voice: &chip8::song::Voice,
    preset_map: &HashMap<String, String>,
    total_steps: usize,
    steps_per_beat: usize,
) -> TrackOverview {
    let display_name = if voice.name.is_empty() {
        format!("track_{:02}", index + 1)
    } else {
        voice.name.clone()
    };
    let preset = preset_map
        .get(&voice.instrument)
        .cloned()
        .unwrap_or_default();
    let instrument = if voice.instrument.is_empty() {
        "direct".to_string()
    } else {
        voice.instrument.clone()
    };

    let mut activity = vec![StepState::default(); total_steps];
    for (step_index, &frequency) in voice.steps.iter().enumerate() {
        if step_index >= activity.len() || frequency <= 0.0 {
            continue;
        }

        let accent = step_index % steps_per_beat == 0;
        activity[step_index] = StepState {
            active: true,
            accent,
            fx_trigger: accent,
        };
    }

    TrackOverview {
        name: display_name,
        instrument,
        preset,
        waveform: waveform_name(voice.waveform).to_string(),
        amplitude: voice.amplitude,
        duty_cycle: voice.duty_cycle,
        attack_ms: voice.attack_ms,
        decay_ms: voice.decay_ms,
        sustain_level: voice.sustain_level,
        release_ms: voice.release_ms,
        gate_percent: voice.gate_percent,
        vibrato_cents: voice.vibrato_cents,
        vibrato_rate: if voice.vibrato_rate > 0 {
            voice.vibrato_rate
        } else {
            ABC_DEFAULT_VIBRATO_RATE
        },
        glide_ms: voice.glide_ms,
        // The ABC GUI metadata path does not expose detune; keep the inspector explicit
        // about the current read-only value instead of inventing a synthetic parameter.
        detune_cents: 0,
        fx_bus: voice.fx_bus.max(0) as usize,
        activity,
    }
}

/// The buses the score declared, or as many as its voices send to, at
/// least one, at most what the engine has.
fn shown_fx_buses(song: &Song) -> &[FxBus] {
    let highest_voice_bus = song
        .voices
        .iter()
        .map(|voice| voice.fx_bus.max(0) as usize)
        .max()
        .unwrap_or(0);
    let count = song
        .fx_bus_count
        .max(highest_voice_bus + 1)
        .max(1)
        .min(song.fx_buses.len());
    &song.fx_buses[..count]
}

fn build_fx_buses(song: &Song) -> Vec<FxBusOverview> {
    shown_fx_buses(song)
        .iter()
        .enumerate()
        .map(|(bus_index, bus)| FxBusOverview {
            bus_index,
            enabled: bus.enabled
                || bus.delay_steps > 0
                || bus.drive_amount > 0
                || bus.lowpass_amount > 0
                || bus.sidechain_amount > 0,
            delay_steps: bus.delay_steps,
            delay_feedback: bus.delay_feedback,
            delay_mix: bus.delay_mix,
            drive_amount: bus.drive_amount,
            lowpass_amount: bus.lowpass_amount,
            sidechain_amount: bus.sidechain_amount,
            sidechain_release_ms: if bus.sidechain_release_ms > 0 {
                bus.sidechain_release_ms
            } else {
                ABC_DEFAULT_SIDECHAIN_RELEASE_MS
            },
            mix_percent: if bus.mix_percent > 0 {
                bus.mix_percent
            } else {
                100
            },
        })
        .collect()
}

fn build_instrument_preset_map(song: &Song) -> HashMap<String, String> {
    song.instruments
        .iter()
        .filter(|instrument| !instrument.name.is_empty())
        .map(|instrument| (instrument.name.clone(), instrument.preset.clone()))
        .collect()
}

fn build_arrangement(song: &Song, max_steps: usize) -> Result<Vec<PatternBlock>, String> {
    let mut blocks = Vec::new();
    let mut start_step = 0usize;

    if !song.patterns.is_empty() && !song.arrangement.is_empty() {
        let lengths = pattern_lengths(song);
        for label in &song.arrangement {
            let Some(&length) = lengths.get(label) else {
                return Err(format!(
                    "arrangement references undefined pattern '{label}'"
                ));
            };
            blocks.push(PatternBlock {
                label: label.clone(),
                length,
                start_step,
            });
            start_step += length;
        }
    } else if !song.patterns.is_empty() {
        for pattern in &song.patterns {
            let length = normalized_pattern_length(pattern.length);
            blocks.push(PatternBlock {
                label: pattern.name.clone(),
                length,
                start_step,
            });
            start_step += length;
        }
    } else if max_steps > 0 {
        let mut chunk_index = 0usize;
        while start_step < max_steps {
            let length = usize::min(16, max_steps - start_step);
            blocks.push(PatternBlock {
                label: format!("P{}", chunk_index + 1),
                length,
                start_step,
            });
            start_step += length;
            chunk_index += 1;
        }
    }

    while start_step < max_steps {
        let length = usize::min(16, max_steps - start_step);
        blocks.push(PatternBlock {
            label: format!("P{}", blocks.len() + 1),
            length,
            start_step,
        });
        start_step += length;
    }

    Ok(blocks)
}

fn pattern_lengths(song: &Song) -> HashMap<String, usize> {
    song.patterns
        .iter()
        .filter(|pattern| pattern.defined)
        .map(|pattern| {
            (
                pattern.name.clone(),
                normalized_pattern_length(pattern.length),
            )
        })
        .collect()
}

fn normalized_pattern_length(length: i32) -> usize {
    length.clamp(1, 64) as usize
}

fn inferred_steps_per_beat(song: &Song) -> usize {
    if song.bpm <= 0 || song.step_ms <= 0 {
        return 4;
    }

    ((60_000.0 / song.bpm as f32) / song.step_ms as f32)
        .round()
        .clamp(1.0, 16.0) as usize
}

fn waveform_name(waveform: Waveform) -> &'static str {
    match waveform {
        Waveform::Pulse => "pulse",
        Waveform::Triangle => "triangle",
        Waveform::Noise => "noise",
        Waveform::Square | Waveform::Other(_) => "square",
    }
}

/// The editor's waveform codes (the ABC dialect's order).
fn waveform_code(waveform: Waveform) -> i32 {
    match waveform {
        Waveform::Square => 0,
        Waveform::Pulse => 1,
        Waveform::Triangle => 2,
        Waveform::Noise => 3,
        Waveform::Other(code) => code,
    }
}

// ─── Editor song loading ─────────────────────────────────────────────────────

/// Instrument data in a form the editor module can consume.
pub struct RawInstrumentForEditor {
    pub name: String,
    pub preset: String,
    pub waveform: i32,
    pub amplitude: i32,
    pub duty_cycle: i32,
    pub attack_ms: i32,
    pub decay_ms: i32,
    pub sustain_level: i32,
    pub release_ms: i32,
    pub gate_percent: i32,
    pub vibrato_cents: i32,
    pub glide_ms: i32,
    pub fx_bus: i32,
}

/// FX bus data in a form the editor module can consume.
pub struct RawFxBusForEditor {
    pub delay_steps: i32,
    pub delay_feedback: i32,
    pub delay_mix: i32,
    pub drive_amount: i32,
    pub lowpass_amount: i32,
    pub sidechain_amount: i32,
    pub sidechain_release_ms: i32,
    pub mix_percent: i32,
    pub ladder_amount: i32,
    pub ladder_cutoff: i32,
    pub ladder_resonance: i32,
}

/// Pattern definition in a form the editor module can consume.
pub struct RawPatternForEditor {
    pub name: String,
    pub length: usize,
}

/// Voice/track data including the raw note frequencies.
pub struct RawVoiceForEditor {
    pub name: String,
    pub instrument_ref: String,
    pub gate_percent: i32,
    /// Per-step note frequencies in Hz; 0.0 means rest.
    pub note_freqs: Vec<f64>,
}

/// Full song data, consumed by the Rust editor module.
pub struct RawAbcMusicForEditor {
    pub title: String,
    pub bpm: i32,
    pub swing_pct: i32,
    pub instruments: Vec<RawInstrumentForEditor>,
    pub fx_buses: Vec<RawFxBusForEditor>,
    pub patterns: Vec<RawPatternForEditor>,
    pub arrangement: Vec<String>,
    pub voices: Vec<RawVoiceForEditor>,
}

/// Load raw editor song data from an ABC file, preserving per-step note
/// frequencies so the editor model can reconstruct MIDI pitches.
pub fn load_raw_abc_music_for_editor(path: &Path) -> Result<RawAbcMusicForEditor, String> {
    let song = load_song(path).map_err(|err| format!("editor load: {err}"))?;
    Ok(extract_raw_abc_music(song))
}

fn extract_raw_abc_music(song: Song) -> RawAbcMusicForEditor {
    let fx_buses = shown_fx_buses(&song)
        .iter()
        .map(|bus| RawFxBusForEditor {
            delay_steps: bus.delay_steps,
            delay_feedback: bus.delay_feedback,
            delay_mix: bus.delay_mix,
            drive_amount: bus.drive_amount,
            lowpass_amount: bus.lowpass_amount,
            sidechain_amount: bus.sidechain_amount,
            sidechain_release_ms: bus.sidechain_release_ms,
            mix_percent: bus.mix_percent,
            ladder_amount: bus.ladder_amount,
            ladder_cutoff: bus.ladder_cutoff,
            ladder_resonance: bus.ladder_resonance,
        })
        .collect();

    let instruments = song
        .instruments
        .into_iter()
        .map(|inst| RawInstrumentForEditor {
            name: inst.name,
            preset: inst.preset,
            waveform: waveform_code(inst.waveform),
            amplitude: inst.amplitude,
            duty_cycle: inst.duty_cycle,
            attack_ms: inst.attack_ms,
            decay_ms: inst.decay_ms,
            sustain_level: inst.sustain_level,
            release_ms: inst.release_ms,
            gate_percent: inst.gate_percent,
            vibrato_cents: inst.vibrato_cents,
            glide_ms: inst.glide_ms,
            fx_bus: inst.fx_bus,
        })
        .collect();

    let patterns = song
        .patterns
        .into_iter()
        .filter(|p| p.defined)
        .map(|p| RawPatternForEditor {
            length: normalized_pattern_length(p.length),
            name: p.name,
        })
        .collect();

    let arrangement = song
        .arrangement
        .into_iter()
        .filter(|slot| !slot.is_empty())
        .collect();

    let voices = song
        .voices
        .into_iter()
        .map(|voice| RawVoiceForEditor {
            name: voice.name,
            instrument_ref: voice.instrument,
            gate_percent: voice.gate_percent,
            note_freqs: voice.steps,
        })
        .collect();

    RawAbcMusicForEditor {
        title: song.title,
        bpm: song.bpm,
        swing_pct: song.swing_pct,
        instruments,
        fx_buses,
        patterns,
        arrangement,
        voices,
    }
}
