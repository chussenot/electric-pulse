#ifndef ELECTRIC_PULSE_H
#define ELECTRIC_PULSE_H

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <ctype.h>
#include <stdint.h>
#include <time.h>

#include "audio_seq.h"
#include "audio_engine.h"

/* abc.c — ABC notation parser and PCM generator */
#define ABC_MAX_VOICES     8
#define ABC_MAX_NOTES      1024
#define ABC_MAX_INSTRUMENTS 8
#define ABC_MAX_PATTERNS   16
#define ABC_MAX_ARRANGEMENT 32
#define ABC_MAX_FX_BUSES   4
#define SAMPLE_RATE_ABC    22050

/* Named instrument preset */
typedef struct {
    char name[32];        /* instrument name */
    char preset[32];      /* preset identifier */
    int amplitude;
    int waveform;
    int duty_cycle;
    int attack_ms;
    int decay_ms;
    int sustain_level;
    int release_ms;
    int gate_percent;
    int vibrato_cents;
    int glide_ms;
    int fx_bus;          /* which FX bus (0-3) */
} AbcInstrument;

/* Pattern definition */
typedef struct {
    char name[32];       /* pattern name (A, B, C, etc.) */
    int length;          /* pattern length in steps */
    int defined;         /* 1 if pattern has been defined */
} AbcPattern;

/* FX bus configuration */
typedef struct {
    int enabled;
    int delay_steps;
    int delay_feedback;
    int delay_mix;
    int drive_amount;
    int lowpass_amount;
    int sidechain_amount;
    int sidechain_release_ms;
    int mix_percent;     /* bus output mix (0-100) */
    int ladder_amount;   /* moog ladder wet mix 0-100; 0 = disabled */
    int ladder_cutoff;   /* 1-100 (percent of Nyquist) */
    int ladder_resonance;/* 0-100 */
} AbcFxBus;

typedef struct {
    char name[32];
    char instrument_ref[32]; /* reference to instrument name, if any */
    int amplitude;        /* per-voice amplitude (0-127) */
    int staccato;         /* 1 = staccato (3/4 length), 0 = legato (9/10 length) */
    int waveform;         /* 0=square, 1=pulse, 2=triangle, 3=noise */
    int duty_cycle;       /* pulse duty cycle percent (1-99), ignored otherwise */
    int attack_ms;
    int decay_ms;
    int sustain_level;
    int release_ms;
    int gate_percent;
    int vibrato_cents;
    int vibrato_rate;
    int glide_ms;
    int fx_bus;           /* which FX bus to use (0-3) */
    double freqs[ABC_MAX_NOTES]; /* frequency per step (0 = rest) */
    int note_count;
} AbcVoice;

typedef struct {
    char title[128];
    int bpm;
    int step_ms;          /* duration of one default-length note in ms */
    int swing_pct;        /* swing percentage 0..100 (%%swing) */

    /* Instruments */
    AbcInstrument instruments[ABC_MAX_INSTRUMENTS];
    int instrument_count;

    /* Patterns */
    AbcPattern patterns[ABC_MAX_PATTERNS];
    int pattern_count;

    /* Arrangement */
    char arrangement[ABC_MAX_ARRANGEMENT][32]; /* pattern sequence */
    int arrangement_length;

    /* FX Buses (backward compatible: bus 0 is default) */
    AbcFxBus fx_buses[ABC_MAX_FX_BUSES];
    int fx_bus_count;

    /* Legacy FX fields (mapped to fx_buses[0] for compatibility) */
    int fx_delay_steps;
    int fx_delay_feedback;
    int fx_delay_mix;
    int fx_drive_amount;
    int fx_lowpass_amount;
    int fx_sidechain_amount;
    int fx_sidechain_release_ms;

    AbcVoice voices[ABC_MAX_VOICES];
    int voice_count;
} AbcMusic;

int abc_load(const char *path, AbcMusic *music);
int abc_load_voices(const char *paths[], int path_count, AbcMusic *music);
int abc_build_seq_song(const AbcMusic *music, SeqSong *out_song);
unsigned char *abc_generate_pcm(const AbcMusic *music, int *out_len);

#endif
