#ifndef ELECTRIC_PULSE_MIDI_EXPORT_H
#define ELECTRIC_PULSE_MIDI_EXPORT_H

/*
 * midi_export.h — Read-only SeqSong -> Standard MIDI File exporter.
 *
 * This is an ADJACENT, off-the-render-path bridge (see ADR-0003). It walks an
 * already-built SeqSong's timeline/track/step data and writes a Standard MIDI
 * File (format 1) to disk. It NEVER calls into, mutates, or is called by the
 * render path (audio_engine_render_abc_file / audio_mix / audio_dsp / audio_fx),
 * so adding it leaves the showcase regression goldens bit-identical.
 *
 * No floats are required: MIDI is inherently integer (PPQ ticks, 0..127 note
 * and velocity values, big-endian byte framing). The engine already stores
 * SeqStep.note as a MIDI note number 0..127 and SeqStep.velocity as a 0..127
 * value, so pitch/velocity map straight through with no conversion.
 */

#include "audio_seq.h"

/*
 * Summary filled by midi_export_seq_song(). All counts reflect what was
 * actually written to the file. Pass NULL if you don't care.
 */
typedef struct {
    int track_count;        /* number of MTrk chunks written (one per voice) */
    int note_count;         /* total note-on/note-off PAIRS written */
    int ticks_per_quarter;  /* PPQ in the MThd header */
    int tempo_bpm;          /* BPM the tempo meta-event was derived from */
} MidiExportStats;

/*
 * Export `song` to a Standard MIDI File at `path`.
 * Returns 0 on success, non-zero on error (NULL args, no renderable steps,
 * file open/write failure). `out_stats` may be NULL.
 */
int midi_export_seq_song(const SeqSong *song, const char *path,
                         MidiExportStats *out_stats);

/*
 * Convenience file-in/file-out wrapper for FFI/GUI callers: parse an .abc file
 * at `abc_path`, build its SeqSong, and export a Standard MIDI File to
 * `out_path`. Off the render path (parse + build + export only, no rendering).
 * Returns 0 on success, non-zero on NULL args or parse/build/export failure.
 * `out_stats` may be NULL.
 */
int midi_export_abc_file(const char *abc_path, const char *out_path,
                         MidiExportStats *out_stats);

#endif
