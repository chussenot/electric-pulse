/*
 * export_midi - CLI driver for the read-only SeqSong -> MIDI exporter.
 *
 * Resolves a demo name (or a .abc path) -> abc_load -> abc_build_seq_song ->
 * midi_export_seq_song. This is the off-the-render-path bridge from ADR-0003;
 * it never calls audio_engine_render_*.
 *
 * Usage: export-midi <name|path.abc> [out.mid]
 * Default output path: bin/midi/<name>.mid
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "../src/electric_pulse.h"
#include "../src/audio_seq.h"
#include "../src/midi_export.h"

typedef struct {
    const char *name;
    const char *path;
} DemoEntry;

/* Kept in sync with tests/play_demo.c kDemos so name resolution matches. */
static const DemoEntry kDemos[] = {
    { "dark_moroder", "data/music/dark_moroder.abc" },
    { "perturbator_loop", "data/music/perturbator_loop.abc" },
    { "carpenter_drive", "data/music/carpenter_drive.abc" },
    { "advanced_dsl_demo", "data/music/advanced_dsl_demo.abc" },
    { "multi_fx_demo", "data/music/multi_fx_demo.abc" },
    { "neon_nightdrive", "data/music/neon_nightdrive.abc" },
    { "metro_chase", "data/music/metro_chase.abc" },
    { "black_sunrise", "data/music/black_sunrise.abc" },
    { "machine_romance", "data/music/machine_romance.abc" },
    { "hypersleep_dream", "data/music/hypersleep_dream.abc" },
    { "aurora_halo", "data/music/aurora_halo.abc" },
    { "glass_anthem", "data/music/glass_anthem.abc" },
    { "pixie_dust", "data/music/pixie_dust.abc" },
    { "surrender_loop", "data/music/surrender_loop.abc" },
    { "moog_lattice", "data/music/moog_lattice.abc" },
    { "three_chord_howl", "data/music/three_chord_howl.abc" }
};

static const char *resolve_demo_path(const char *demo)
{
    size_t i;
    if (!demo || !*demo) return NULL;
    for (i = 0; i < sizeof(kDemos) / sizeof(kDemos[0]); i++) {
        if (strcmp(kDemos[i].name, demo) == 0) return kDemos[i].path;
    }
    if (strstr(demo, ".abc") != NULL || strchr(demo, '/') != NULL) return demo;
    return NULL;
}

static void usage(void)
{
    size_t i;
    printf("Usage: export-midi <name|path.abc> [out.mid]\n");
    printf("Available demos:\n");
    for (i = 0; i < sizeof(kDemos) / sizeof(kDemos[0]); i++)
        printf("  - %s\n", kDemos[i].name);
}

int main(int argc, char **argv)
{
    const char *path;
    const char *out_path;
    char default_out[512];
    AbcMusic music;
    SeqSong song;
    MidiExportStats stats;

    if (argc < 2) {
        usage();
        return 1;
    }

    path = resolve_demo_path(argv[1]);
    if (!path) {
        fprintf(stderr, "Unknown demo: %s\n", argv[1]);
        usage();
        return 1;
    }

    if (argc >= 3) {
        out_path = argv[2];
    } else {
        /* bin/midi/<argv[1]>.mid; sanitize: use the bare name as given. */
        mkdir("bin", 0777);
        mkdir("bin/midi", 0777);
        snprintf(default_out, sizeof(default_out), "bin/midi/%s.mid", argv[1]);
        out_path = default_out;
    }

    if (abc_load(path, &music) != 0) {
        fprintf(stderr, "Could not parse: %s\n", path);
        return 1;
    }
    if (abc_build_seq_song(&music, &song) != 0) {
        fprintf(stderr, "Could not build SeqSong: %s\n", path);
        return 1;
    }
    if (midi_export_seq_song(&song, out_path, &stats) != 0) {
        fprintf(stderr, "MIDI export failed: %s\n", out_path);
        return 1;
    }

    printf("%s -> %s | tracks=%d | notes=%d | ppq=%d | bpm=%d\n",
           music.title[0] ? music.title : path,
           out_path,
           stats.track_count,
           stats.note_count,
           stats.ticks_per_quarter,
           stats.tempo_bpm);
    return 0;
}
