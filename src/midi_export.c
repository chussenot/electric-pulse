/*
 * midi_export.c — Read-only SeqSong -> Standard MIDI File (format 1).
 *
 * See ADR-0003 (docs/adr/adr-0003-magenta-realtime-2-neural-audio-bridge.md):
 * this is an adjacent bridge that lives OFF the render path. It only reads the
 * SeqSong the parser already produced; it never touches audio_engine_render_*,
 * the mixer, the DSP voices, or the FX chain, and it does no float math.
 *
 * ----------------------------------------------------------------------------
 * Pitch mapping (the crux of correctness)
 * ----------------------------------------------------------------------------
 * SeqStep.note is already a MIDI note number 0..127. The mixer proves this:
 * audio_mix.c calls midi_note_to_freq(event->note) against a 128-entry MIDI
 * frequency table, and abc.c fills SeqStep.note via closest_midi_from_freq().
 * So we emit `step->note` verbatim as the MIDI key. SEQ_NOTE_REST (-1) and
 * velocity == 0 are silence and produce no event (matching
 * seq_collect_step_events, which skips them).
 *
 * SeqStep.velocity is already 0..127, the MIDI velocity range, so it maps
 * straight through (clamped 1..127 for the note-on; a 0 note-on would mean
 * note-off in MIDI, and a rested step is skipped anyway).
 *
 * ----------------------------------------------------------------------------
 * Timing mapping
 * ----------------------------------------------------------------------------
 * The sequencer grid: `steps_per_beat` timeline steps == one quarter-note beat
 * (a beat is the Q: reference note). One timeline step is therefore a
 * quarter-note / steps_per_beat. We choose a FIXED ticks-per-step and derive
 * PPQ from it:
 *
 *     ticks_per_quarter = MIDI_TICKS_PER_STEP * steps_per_beat
 *
 * This guarantees step boundaries land on integer tick counts for ANY
 * steps_per_beat (1..16) with zero rounding — unlike picking a fixed PPQ such
 * as 480 and dividing, which is inexact for steps_per_beat in {7,9,11,...}.
 * With the common steps_per_beat == 4 this yields the conventional PPQ 480.
 *
 * A note's tick duration is the gate percent applied to one step length,
 * mirroring how the mixer derives gate_samples = duration_samples * gate / 100.
 * We deliberately do NOT bake the engine's swing micro-timing into the tick
 * grid: swing is a render-time groove; encoding it would push events off the
 * DAW/Magenta beat grid and break clean round-tripping. Symbolic MIDI keeps a
 * straight grid (step k starts at k * ticks_per_step).
 *
 * Tempo: a single Set-Tempo meta-event of 60_000_000 / bpm microseconds per
 * quarter note, derived from SeqSong.tempo_bpm.
 *
 * ----------------------------------------------------------------------------
 * File shape (SMF format 1)
 * ----------------------------------------------------------------------------
 *   MThd: format=1, ntrks=1 (conductor) + N voice tracks, division=PPQ
 *   Track 0 (conductor): track name + Set-Tempo + End-of-Track
 *   Track i (1..N): one MTrk per SeqSong voice with that voice's note events
 * All multi-byte header/meta integers are big-endian; delta times are VLQ.
 */

#include "midi_export.h"
#include "electric_pulse.h"  /* AbcMusic, abc_load, abc_build_seq_song (wrapper only) */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Fixed ticks granted to a single timeline step. PPQ is derived from this so
 * step boundaries are always integer ticks (see header-comment timing notes). */
#define MIDI_TICKS_PER_STEP 120

/* All voice notes go on MIDI channel 0; the engine has no channel concept. */
#define MIDI_CHANNEL 0

/* A growable byte buffer so we can compute each MTrk's exact length before
 * writing its 4-byte big-endian length field. */
typedef struct {
    uint8_t *data;
    size_t   len;
    size_t   cap;
    int      oom; /* sticky out-of-memory flag */
} ByteBuf;

static void bb_init(ByteBuf *b)
{
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->oom = 0;
}

static void bb_free(ByteBuf *b)
{
    free(b->data);
    b->data = NULL;
    b->len = b->cap = 0;
}

static int bb_reserve(ByteBuf *b, size_t extra)
{
    if (b->oom) return -1;
    if (b->len + extra <= b->cap) return 0;
    size_t want = b->cap ? b->cap : 256;
    while (want < b->len + extra) want *= 2;
    uint8_t *p = realloc(b->data, want);
    if (!p) { b->oom = 1; return -1; }
    b->data = p;
    b->cap = want;
    return 0;
}

static void bb_byte(ByteBuf *b, uint8_t v)
{
    if (bb_reserve(b, 1) != 0) return;
    b->data[b->len++] = v;
}

static void bb_bytes(ByteBuf *b, const void *src, size_t n)
{
    if (bb_reserve(b, n) != 0) return;
    memcpy(b->data + b->len, src, n);
    b->len += n;
}

/* MIDI variable-length quantity: 7 bits per byte, big-endian, high bit set on
 * every byte except the last. Encodes delta times and meta-event lengths. */
static void bb_vlq(ByteBuf *b, uint32_t value)
{
    uint8_t stack[5];
    int n = 0;
    /* Emit at least one byte even for 0. */
    stack[n++] = (uint8_t)(value & 0x7f);
    value >>= 7;
    while (value > 0) {
        stack[n++] = (uint8_t)((value & 0x7f) | 0x80);
        value >>= 7;
    }
    /* Bytes were produced least-significant-first; write most-significant-first. */
    for (int i = n - 1; i >= 0; i--)
        bb_byte(b, stack[i]);
}

/* One scheduled MIDI event on a track, keyed by absolute tick. We collect all
 * note-on/off events per track, sort by tick (note-off before note-on at the
 * same tick so a re-struck pitch releases cleanly), then emit VLQ deltas. */
typedef struct {
    uint32_t tick;
    uint8_t  status;   /* 0x90 note-on | 0x80 note-off (channel folded in) */
    uint8_t  note;
    uint8_t  velocity;
    int      order;    /* stable-sort tiebreak preserving insertion order */
} MidiEvent;

static int event_cmp(const void *pa, const void *pb)
{
    const MidiEvent *a = pa;
    const MidiEvent *b = pb;
    if (a->tick != b->tick)
        return (a->tick < b->tick) ? -1 : 1;
    /* At an identical tick, note-off (0x80) sorts before note-on (0x90) so a
     * repeated pitch is released before being struck again. */
    int a_off = (a->status & 0xf0) == 0x80;
    int b_off = (b->status & 0xf0) == 0x80;
    if (a_off != b_off)
        return a_off ? -1 : 1;
    if (a->order != b->order)
        return (a->order < b->order) ? -1 : 1;
    return 0;
}

static int clampi(int v, int lo, int hi)
{
    if (v < lo) return lo;
    if (v > hi) return hi;
    return v;
}

/* Append the conductor track (track 0): track-name + Set-Tempo + End-of-Track.
 * Returns 0 on success. */
static int write_conductor_track(FILE *f, const SeqSong *song)
{
    ByteBuf trk;
    bb_init(&trk);

    /* delta 0, meta 0x03 Sequence/Track Name */
    {
        const char *name = (song->title[0] != '\0') ? song->title : "Electric Pulse";
        size_t nlen = strlen(name);
        if (nlen > 127) nlen = 127; /* keep the VLQ length single-byte and tidy */
        bb_vlq(&trk, 0);
        bb_byte(&trk, 0xff);
        bb_byte(&trk, 0x03);
        bb_vlq(&trk, (uint32_t)nlen);
        bb_bytes(&trk, name, nlen);
    }

    /* delta 0, meta 0x51 Set Tempo, len 3, microseconds-per-quarter (24-bit BE). */
    {
        int bpm = song->tempo_bpm > 0 ? song->tempo_bpm : 120;
        uint32_t us_per_quarter = (uint32_t)(60000000 / bpm);
        bb_vlq(&trk, 0);
        bb_byte(&trk, 0xff);
        bb_byte(&trk, 0x51);
        bb_byte(&trk, 0x03);
        bb_byte(&trk, (uint8_t)((us_per_quarter >> 16) & 0xff));
        bb_byte(&trk, (uint8_t)((us_per_quarter >> 8) & 0xff));
        bb_byte(&trk, (uint8_t)(us_per_quarter & 0xff));
    }

    /* delta 0, meta 0x2f End of Track, len 0. */
    bb_vlq(&trk, 0);
    bb_byte(&trk, 0xff);
    bb_byte(&trk, 0x2f);
    bb_byte(&trk, 0x00);

    if (trk.oom) { bb_free(&trk); return -1; }

    fwrite("MTrk", 1, 4, f);
    {
        uint8_t lenbuf[4] = {
            (uint8_t)((trk.len >> 24) & 0xff), (uint8_t)((trk.len >> 16) & 0xff),
            (uint8_t)((trk.len >> 8) & 0xff),  (uint8_t)(trk.len & 0xff)
        };
        fwrite(lenbuf, 1, 4, f);
    }
    fwrite(trk.data, 1, trk.len, f);
    bb_free(&trk);
    return 0;
}

/*
 * Collect every note event for one SeqSong voice index across the whole
 * arrangement timeline, then write it as a single MTrk. Returns the number of
 * note PAIRS written (>= 0) on success, or -1 on allocation/write failure.
 *
 * We re-walk the arrangement exactly like seq_compile_timeline does (linear
 * consumption: each slot advances by its pattern length), so the per-voice
 * note ordering and onset positions match what the engine renders. Timing is
 * expressed in clean musical ticks rather than the mixer's swing-adjusted
 * sample counts (see file-header note).
 */
static int write_voice_track(FILE *f, const SeqSong *song, int voice_index,
                             int ticks_per_step)
{
    MidiEvent *events = NULL;
    size_t ev_count = 0, ev_cap = 0;
    int pair_count = 0;
    int order = 0;
    uint32_t step_base_tick = 0;
    int rc = 0;

    /* Walk arrangement slots in order; absolute tick advances per step. */
    for (int slot = 0; slot < song->arrangement_length; slot++) {
        int pattern_index = song->arrangement[slot];
        if (pattern_index < 0 || pattern_index >= song->pattern_count)
            continue;
        const SeqPattern *pattern = &song->patterns[pattern_index];

        for (int s = 0; s < pattern->length && s < SEQ_MAX_STEPS; s++) {
            uint32_t step_tick = step_base_tick + (uint32_t)s * (uint32_t)ticks_per_step;

            /* Only this voice's track within the pattern (if it has one). */
            if (voice_index >= pattern->track_count)
                continue;
            const SeqTrack *track = &pattern->tracks[voice_index];
            const SeqStep *step = &track->steps[s];

            /* Mirror seq_collect_step_events' skip rules exactly. */
            if (track->instrument < 0 || track->instrument >= song->instrument_count)
                continue;
            if (step->note == SEQ_NOTE_REST || step->velocity == 0)
                continue;
            if (step->note < 0 || step->note > 127)
                continue;

            /* Gate: same derivation as the mixer (gate% of one step length),
             * defaulting to the instrument envelope gate when the step is 0. */
            const SeqInstrument *inst = &song->instruments[track->instrument];
            int gate = (step->gate > 0) ? step->gate : inst->envelope.gate_percent;
            gate = clampi(gate, 1, 100);
            uint32_t dur = ((uint32_t)ticks_per_step * (uint32_t)gate) / 100u;
            if (dur < 1) dur = 1; /* never emit a zero-length note */

            int vel = clampi((int)step->velocity, 1, 127);

            if (ev_count + 2 > ev_cap) {
                size_t newcap = ev_cap ? ev_cap * 2 : 64;
                MidiEvent *p = realloc(events, newcap * sizeof(*events));
                if (!p) { rc = -1; goto done; }
                events = p;
                ev_cap = newcap;
            }

            events[ev_count].tick = step_tick;
            events[ev_count].status = (uint8_t)(0x90 | MIDI_CHANNEL);
            events[ev_count].note = (uint8_t)step->note;
            events[ev_count].velocity = (uint8_t)vel;
            events[ev_count].order = order++;
            ev_count++;

            events[ev_count].tick = step_tick + dur;
            events[ev_count].status = (uint8_t)(0x80 | MIDI_CHANNEL);
            events[ev_count].note = (uint8_t)step->note;
            events[ev_count].velocity = 0;
            events[ev_count].order = order++;
            ev_count++;

            pair_count++;
        }

        step_base_tick += (uint32_t)pattern->length * (uint32_t)ticks_per_step;
    }

    /* Stable-sort by (tick, off-before-on, insertion order). */
    if (ev_count > 0)
        qsort(events, ev_count, sizeof(*events), event_cmp);

    /* Serialize to a track buffer so we can prefix the exact length. */
    {
        ByteBuf trk;
        bb_init(&trk);

        /* Track name = voice index label; deterministic and DAW-friendly. */
        {
            char name[32];
            int nlen = snprintf(name, sizeof(name), "Voice %d", voice_index + 1);
            if (nlen < 0) nlen = 0;
            if (nlen > (int)sizeof(name)) nlen = (int)sizeof(name);
            bb_vlq(&trk, 0);
            bb_byte(&trk, 0xff);
            bb_byte(&trk, 0x03);
            bb_vlq(&trk, (uint32_t)nlen);
            bb_bytes(&trk, name, (size_t)nlen);
        }

        uint32_t prev_tick = 0;
        for (size_t i = 0; i < ev_count; i++) {
            uint32_t delta = events[i].tick - prev_tick;
            prev_tick = events[i].tick;
            bb_vlq(&trk, delta);
            bb_byte(&trk, events[i].status);
            bb_byte(&trk, events[i].note);
            bb_byte(&trk, events[i].velocity);
        }

        /* End of Track. */
        bb_vlq(&trk, 0);
        bb_byte(&trk, 0xff);
        bb_byte(&trk, 0x2f);
        bb_byte(&trk, 0x00);

        if (trk.oom) { bb_free(&trk); rc = -1; goto done; }

        fwrite("MTrk", 1, 4, f);
        {
            uint8_t lenbuf[4] = {
                (uint8_t)((trk.len >> 24) & 0xff), (uint8_t)((trk.len >> 16) & 0xff),
                (uint8_t)((trk.len >> 8) & 0xff),  (uint8_t)(trk.len & 0xff)
            };
            fwrite(lenbuf, 1, 4, f);
        }
        fwrite(trk.data, 1, trk.len, f);
        bb_free(&trk);
    }

done:
    free(events);
    return (rc == 0) ? pair_count : -1;
}

/* The number of distinct voices/tracks the arrangement actually references.
 * Equals the max track_count across the patterns used, mirroring the mixer's
 * timeline->max_track_count. */
static int song_voice_count(const SeqSong *song)
{
    int max_tracks = 0;
    for (int slot = 0; slot < song->arrangement_length; slot++) {
        int pi = song->arrangement[slot];
        if (pi < 0 || pi >= song->pattern_count) continue;
        int tc = song->patterns[pi].track_count;
        if (tc > max_tracks) max_tracks = tc;
    }
    if (max_tracks > SEQ_MAX_TRACKS) max_tracks = SEQ_MAX_TRACKS;
    return max_tracks;
}

int midi_export_seq_song(const SeqSong *song, const char *path,
                         MidiExportStats *out_stats)
{
    if (!song || !path) return 1;

    int voice_count = song_voice_count(song);
    if (voice_count <= 0 || song->arrangement_length <= 0)
        return 1; /* nothing renderable -> nothing to export */

    int steps_per_beat = song->steps_per_beat > 0 ? song->steps_per_beat : 4;
    int ticks_per_step = MIDI_TICKS_PER_STEP;
    int ppq = ticks_per_step * steps_per_beat;
    /* ppq must fit the 16-bit MThd division field; clamp the (theoretical)
     * extreme of steps_per_beat == 16 -> 1920, well within range. */
    if (ppq < 1) ppq = 1;
    if (ppq > 32767) ppq = 32767;

    FILE *f = fopen(path, "wb");
    if (!f) return 1;

    /* MThd: chunk id, length 6, format 1, ntrks = conductor + voices, division. */
    {
        uint16_t format = 1;
        uint16_t ntrks = (uint16_t)(voice_count + 1);
        uint16_t division = (uint16_t)ppq;
        uint8_t header[14];
        memcpy(header, "MThd", 4);
        header[4] = 0; header[5] = 0; header[6] = 0; header[7] = 6; /* length 6 */
        header[8]  = (uint8_t)((format >> 8) & 0xff);
        header[9]  = (uint8_t)(format & 0xff);
        header[10] = (uint8_t)((ntrks >> 8) & 0xff);
        header[11] = (uint8_t)(ntrks & 0xff);
        header[12] = (uint8_t)((division >> 8) & 0xff);
        header[13] = (uint8_t)(division & 0xff);
        if (fwrite(header, 1, sizeof(header), f) != sizeof(header)) {
            fclose(f);
            return 1;
        }
    }

    if (write_conductor_track(f, song) != 0) {
        fclose(f);
        return 1;
    }

    int total_pairs = 0;
    for (int v = 0; v < voice_count; v++) {
        int pairs = write_voice_track(f, song, v, ticks_per_step);
        if (pairs < 0) {
            fclose(f);
            return 1;
        }
        total_pairs += pairs;
    }

    if (ferror(f)) {
        fclose(f);
        return 1;
    }
    if (fclose(f) != 0)
        return 1;

    if (out_stats) {
        out_stats->track_count = voice_count;
        out_stats->note_count = total_pairs;
        out_stats->ticks_per_quarter = ppq;
        out_stats->tempo_bpm = song->tempo_bpm > 0 ? song->tempo_bpm : 120;
    }
    return 0;
}

/*
 * Convenience file-in/file-out wrapper: parse an .abc file, build its SeqSong,
 * and export to a Standard MIDI File. This is the entry point the GUI/FFI uses
 * so callers never have to manage AbcMusic/SeqSong memory across the boundary.
 * Still off the render path: it only parses + builds + exports, never renders.
 * Returns 0 on success; non-zero on NULL args or parse/build/export failure.
 */
int midi_export_abc_file(const char *abc_path, const char *out_path,
                         MidiExportStats *out_stats)
{
    AbcMusic music;
    SeqSong song;

    if (!abc_path || !out_path)
        return 1;
    if (abc_load(abc_path, &music) != 0)
        return 1;
    if (abc_build_seq_song(&music, &song) != 0)
        return 1;
    return midi_export_seq_song(&song, out_path, out_stats);
}
