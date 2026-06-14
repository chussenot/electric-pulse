use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::model::{
    EditableArrangement, EditableArrangementBlock, EditableFxBus, EditableInstrument,
    EditablePattern, EditableSong, EditableStep, EditableTrack,
};
use super::validation::validate_song;

const MIDI_MAX_TRACKS: usize = 8;
const MIDI_MAX_TOTAL_STEPS: usize = 1024;
const MIDI_PATTERN_STEPS: usize = 64;

#[derive(Clone, Copy, Debug)]
struct MidiNoteEvent {
    start_tick: u32,
    end_tick: u32,
    note: u8,
    velocity: u8,
}

#[derive(Debug)]
struct ParsedMidi {
    ppq: u16,
    tempo_bpm: i32,
    tracks: Vec<Vec<MidiNoteEvent>>,
}

pub fn import_editable_song_from_midi_path(path: &Path) -> Result<EditableSong, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("midi import failed for {}: {error}", path.display()))?;
    let parsed = parse_standard_midi(&bytes)?;
    let mut song = build_editable_song_from_midi(path, parsed)?;
    song.source_path = None;
    song.dirty = true;
    validate_song(&song)?;
    Ok(song)
}

fn build_editable_song_from_midi(path: &Path, parsed: ParsedMidi) -> Result<EditableSong, String> {
    let ticks_per_step = ((parsed.ppq as f64) / 4.0).round().max(1.0);
    let mut total_steps = 16usize;
    for track in &parsed.tracks {
        for event in track {
            let end_step = ((event.end_tick as f64) / ticks_per_step).round() as usize + 1;
            total_steps = total_steps.max(end_step);
        }
    }
    total_steps = total_steps.clamp(16, MIDI_MAX_TOTAL_STEPS);

    let blocks = build_arrangement_blocks(total_steps);
    let patterns = blocks
        .iter()
        .map(|block| EditablePattern {
            name: block.pattern_name.clone(),
            length: block.length,
        })
        .collect::<Vec<_>>();

    let tracks = parsed
        .tracks
        .iter()
        .take(MIDI_MAX_TRACKS)
        .enumerate()
        .map(|(track_index, events)| {
            let mut steps = vec![EditableStep::rest(); total_steps];
            for event in events {
                let start_step = ((event.start_tick as f64) / ticks_per_step).round() as usize;
                if start_step >= steps.len() {
                    continue;
                }
                let duration_ticks = event.end_tick.saturating_sub(event.start_tick).max(1);
                let gate_percent = ((duration_ticks as f64 / ticks_per_step) * 100.0)
                    .round()
                    .clamp(1.0, 100.0) as u8;
                let existing_velocity = steps[start_step].velocity;
                if steps[start_step].active && existing_velocity > event.velocity {
                    continue;
                }
                steps[start_step] = EditableStep {
                    active: true,
                    midi_note: event.note,
                    velocity: event.velocity.clamp(1, 127),
                    gate_percent,
                    accent: event.velocity >= 110,
                    fx_trigger: false,
                };
            }
            EditableTrack {
                name: format!("track_{:02}", track_index + 1),
                instrument_ref: "lead".to_string(),
                steps,
            }
        })
        .collect::<Vec<_>>();

    if tracks.is_empty() {
        return Err("midi import failed: no note events were found".to_string());
    }

    let mut title = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported MIDI")
        .trim()
        .to_string();
    if title.is_empty() {
        title = "Imported MIDI".to_string();
    }

    Ok(EditableSong {
        title,
        tempo: parsed.tempo_bpm.clamp(40, 300),
        swing: 50,
        patterns,
        tracks,
        instruments: vec![EditableInstrument {
            name: "lead".to_string(),
            ..Default::default()
        }],
        fx_buses: vec![EditableFxBus::default_for_index(0)],
        arrangement: EditableArrangement { blocks },
        dirty: true,
        source_path: None,
        comments: Vec::new(),
    })
}

fn build_arrangement_blocks(total_steps: usize) -> Vec<EditableArrangementBlock> {
    let mut blocks = Vec::new();
    let mut remaining = total_steps;
    let mut index = 0usize;
    while remaining > 0 {
        let length = remaining.min(MIDI_PATTERN_STEPS);
        blocks.push(EditableArrangementBlock {
            pattern_name: pattern_name(index),
            length,
        });
        remaining -= length;
        index += 1;
    }
    blocks
}

fn pattern_name(index: usize) -> String {
    if index < 26 {
        ((b'A' + index as u8) as char).to_string()
    } else {
        format!("P{}", index + 1)
    }
}

fn parse_standard_midi(bytes: &[u8]) -> Result<ParsedMidi, String> {
    let mut reader = ByteReader::new(bytes);
    let header_tag = reader.read_bytes(4)?;
    if header_tag != b"MThd" {
        return Err("midi import failed: missing MThd header".to_string());
    }
    let header_len = reader.read_u32_be()? as usize;
    if header_len < 6 {
        return Err("midi import failed: invalid MThd length".to_string());
    }
    let format = reader.read_u16_be()?;
    let track_count = reader.read_u16_be()? as usize;
    let division = reader.read_u16_be()?;
    if division & 0x8000 != 0 {
        return Err("midi import failed: SMPTE time division is not supported".to_string());
    }
    if track_count == 0 {
        return Err("midi import failed: file has zero tracks".to_string());
    }
    if format > 1 {
        return Err(format!(
            "midi import failed: unsupported format {} (expected 0 or 1)",
            format
        ));
    }
    if header_len > 6 {
        reader.skip(header_len - 6)?;
    }

    let mut tracks = Vec::new();
    let mut tempo_bpm = 120i32;
    for _ in 0..track_count {
        let chunk_tag = reader.read_bytes(4)?;
        if chunk_tag != b"MTrk" {
            return Err("midi import failed: expected MTrk chunk".to_string());
        }
        let chunk_len = reader.read_u32_be()? as usize;
        let chunk = reader.read_bytes(chunk_len)?;
        let (events, tempo_hint) = parse_track_events(chunk)?;
        if let Some(tempo) = tempo_hint {
            tempo_bpm = tempo;
        }
        if !events.is_empty() {
            tracks.push(events);
        }
    }

    if tracks.is_empty() {
        return Err("midi import failed: no note events were found".to_string());
    }

    Ok(ParsedMidi {
        ppq: division,
        tempo_bpm,
        tracks,
    })
}

fn parse_track_events(track: &[u8]) -> Result<(Vec<MidiNoteEvent>, Option<i32>), String> {
    let mut reader = ByteReader::new(track);
    let mut absolute_tick: u32 = 0;
    let mut running_status: Option<u8> = None;
    let mut active_notes: HashMap<(u8, u8), (u32, u8)> = HashMap::new();
    let mut notes = Vec::new();
    let mut tempo_bpm = None;

    while !reader.is_done() {
        let delta = reader.read_var_len()?;
        absolute_tick = absolute_tick
            .checked_add(delta)
            .ok_or_else(|| "midi import failed: tick overflow".to_string())?;
        let first = reader.read_u8()?;
        let (status, first_data) = if first & 0x80 != 0 {
            running_status = if first < 0xF0 { Some(first) } else { None };
            (first, None)
        } else {
            let status = running_status
                .ok_or_else(|| "midi import failed: running status without prior status".to_string())?;
            (status, Some(first))
        };

        match status {
            0x80..=0x8F => {
                let note = first_data.unwrap_or(reader.read_u8()?);
                let _velocity = reader.read_u8()?;
                close_note(&mut active_notes, &mut notes, status & 0x0F, note, absolute_tick);
            }
            0x90..=0x9F => {
                let note = first_data.unwrap_or(reader.read_u8()?);
                let velocity = reader.read_u8()?;
                let key = (status & 0x0F, note);
                if velocity == 0 {
                    close_note(&mut active_notes, &mut notes, key.0, key.1, absolute_tick);
                } else {
                    if active_notes.contains_key(&key) {
                        close_note(&mut active_notes, &mut notes, key.0, key.1, absolute_tick);
                    }
                    active_notes.insert(key, (absolute_tick, velocity));
                }
            }
            0xA0..=0xAF | 0xB0..=0xBF | 0xE0..=0xEF => {
                let _ = first_data.unwrap_or(reader.read_u8()?);
                let _ = reader.read_u8()?;
            }
            0xC0..=0xDF => {
                let _ = first_data.unwrap_or(reader.read_u8()?);
            }
            0xFF => {
                let meta_type = reader.read_u8()?;
                let len = reader.read_var_len()? as usize;
                let data = reader.read_bytes(len)?;
                if meta_type == 0x51 && len == 3 && tempo_bpm.is_none() {
                    let us_per_quarter =
                        ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32;
                    if us_per_quarter > 0 {
                        tempo_bpm = Some((60_000_000f64 / us_per_quarter as f64).round() as i32);
                    }
                }
                if meta_type == 0x2F {
                    break;
                }
            }
            0xF0 | 0xF7 => {
                let len = reader.read_var_len()? as usize;
                reader.skip(len)?;
            }
            _ => {
                return Err(format!("midi import failed: unsupported status byte 0x{status:02X}"));
            }
        }
    }

    for ((_, note), (start_tick, velocity)) in active_notes {
        let end_tick = absolute_tick.max(start_tick.saturating_add(1));
        notes.push(MidiNoteEvent {
            start_tick,
            end_tick,
            note,
            velocity,
        });
    }
    notes.sort_by_key(|event| (event.start_tick, event.note));
    Ok((notes, tempo_bpm))
}

fn close_note(
    active_notes: &mut HashMap<(u8, u8), (u32, u8)>,
    notes: &mut Vec<MidiNoteEvent>,
    channel: u8,
    note: u8,
    end_tick: u32,
) {
    let key = (channel, note);
    if let Some((start_tick, velocity)) = active_notes.remove(&key) {
        notes.push(MidiNoteEvent {
            start_tick,
            end_tick: end_tick.max(start_tick.saturating_add(1)),
            note,
            velocity,
        });
    }
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn is_done(&self) -> bool {
        self.index >= self.bytes.len()
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.index >= self.bytes.len() {
            return Err("midi import failed: unexpected EOF".to_string());
        }
        let value = self.bytes[self.index];
        self.index += 1;
        Ok(value)
    }

    fn read_u16_be(&mut self) -> Result<u16, String> {
        let bytes = self.read_bytes(2)?;
        Ok(((bytes[0] as u16) << 8) | bytes[1] as u16)
    }

    fn read_u32_be(&mut self) -> Result<u32, String> {
        let bytes = self.read_bytes(4)?;
        Ok(((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | bytes[3] as u32)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.index + len > self.bytes.len() {
            return Err("midi import failed: unexpected EOF".to_string());
        }
        let slice = &self.bytes[self.index..self.index + len];
        self.index += len;
        Ok(slice)
    }

    fn skip(&mut self, len: usize) -> Result<(), String> {
        let _ = self.read_bytes(len)?;
        Ok(())
    }

    fn read_var_len(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self.read_u8()?;
            value = (value << 7) | (byte & 0x7F) as u32;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err("midi import failed: malformed variable-length integer".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_type0_midi_with_tempo_and_single_note() {
        let midi = vec![
            b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 1, 224, b'M', b'T', b'r', b'k', 0, 0,
            0, 20, 0, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20, 0, 0x90, 60, 100, 0x83, 0x60, 0x80, 60,
            0, 0, 0xFF, 0x2F, 0,
        ];
        let parsed = parse_standard_midi(&midi).expect("midi should parse");
        assert_eq!(parsed.ppq, 480);
        assert_eq!(parsed.tempo_bpm, 120);
        assert_eq!(parsed.tracks.len(), 1);
        assert_eq!(parsed.tracks[0].len(), 1);
        assert_eq!(parsed.tracks[0][0].note, 60);
        assert_eq!(parsed.tracks[0][0].velocity, 100);
    }

    #[test]
    fn parse_rejects_smpte_division() {
        let midi = vec![
            b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0xE7, 0x28, b'M', b'T', b'r', b'k', 0,
            0, 0, 4, 0, 0xFF, 0x2F, 0,
        ];
        let error = parse_standard_midi(&midi).expect_err("smpte should be rejected");
        assert!(error.contains("SMPTE"));
    }

    #[test]
    fn import_builds_editable_song_shape() {
        let parsed = ParsedMidi {
            ppq: 480,
            tempo_bpm: 98,
            tracks: vec![vec![MidiNoteEvent {
                start_tick: 0,
                end_tick: 480,
                note: 64,
                velocity: 120,
            }]],
        };
        let song =
            build_editable_song_from_midi(Path::new("/tmp/test.mid"), parsed).expect("imported song");
        assert_eq!(song.tempo, 98);
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].steps.len(), 16);
        assert!(song.tracks[0].steps[0].active);
        assert_eq!(song.tracks[0].steps[0].midi_note, 64);
        assert!(song.tracks[0].steps[0].accent);
    }
}
