mod app;
mod audio_engine;
mod editor;
mod ffi;
mod playback;

use app::ElectricPulseGuiApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("ELECTRIC PULSE SOUND MACHINE"),
        ..Default::default()
    };

    eframe::run_native(
        "ELECTRIC PULSE SOUND MACHINE",
        native_options,
        Box::new(|cc| {
            ElectricPulseGuiApp::configure_visuals(&cc.egui_ctx);
            Ok(Box::new(ElectricPulseGuiApp::default()))
        }),
    )
}
