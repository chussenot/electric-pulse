#ifndef ELECTRIC_PULSE_AUDIO_SONG_BUILTIN_H
#define ELECTRIC_PULSE_AUDIO_SONG_BUILTIN_H

#include "audio_seq.h"

typedef enum {
    ELECTRIC_PULSE_PRESET_BASS_PULSE = 0,
    ELECTRIC_PULSE_PRESET_DARK_ARP,
    ELECTRIC_PULSE_PRESET_SOFT_PAD,
    ELECTRIC_PULSE_PRESET_BRASS_STAB,
    ELECTRIC_PULSE_PRESET_LEAD,
    ELECTRIC_PULSE_PRESET_KICK,
    ELECTRIC_PULSE_PRESET_HAT,
    ELECTRIC_PULSE_PRESET_NOISE_SNARE,
    ELECTRIC_PULSE_PRESET_COUNT
} ElectricPulseInstrumentPreset;

const SeqSong *audio_builtin_menu_song(void);
const SeqInstrument *audio_builtin_instrument_presets(int *count);
const char *audio_builtin_instrument_preset_name(int index);

#endif
