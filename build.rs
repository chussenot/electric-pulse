fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let c_sources = [
        "src/audio_engine.c",
        "src/audio_mix.c",
        "src/audio_seq.c",
        "src/audio_dsp.c",
        "src/audio_fx.c",
        "src/audio_song_builtin.c",
        "src/audio_jam.c",
        "src/abc.c",
        // Read-only SeqSong -> Standard MIDI File exporter (ADR-0003).
        // Off the render path; compiled in so the GUI can call
        // midi_export_abc_file over FFI.
        "src/midi_export.c",
    ];

    for file in c_sources {
        println!("cargo:rerun-if-changed={file}");
    }
    println!("cargo:rerun-if-changed=src/electric_pulse.h");
    println!("cargo:rerun-if-changed=src/audio_engine.h");
    println!("cargo:rerun-if-changed=src/audio_mix.h");
    println!("cargo:rerun-if-changed=src/audio_seq.h");
    println!("cargo:rerun-if-changed=src/audio_dsp.h");
    println!("cargo:rerun-if-changed=src/audio_fx.h");
    println!("cargo:rerun-if-changed=src/audio_song_builtin.h");
    println!("cargo:rerun-if-changed=src/audio_jam.h");
    println!("cargo:rerun-if-changed=src/midi_export.h");

    let mut build = cc::Build::new();
    build
        .include("src")
        .define("_DEFAULT_SOURCE", None)
        .define("_XOPEN_SOURCE", Some("600"))
        .flag_if_supported("-std=c99")
        .warnings(true)
        .extra_warnings(true)
        .files(c_sources);

    build.compile("electric_pulse_audio_engine");

    println!("cargo:rustc-link-lib=m");
}
