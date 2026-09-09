#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod system_fonts;
mod theme;
mod widgets;

use app::Workspace;

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("DevToys")
        .with_app_id("devtoys")
        .with_inner_size([1120.0, 720.0])
        .with_min_inner_size([760.0, 520.0]);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "DevToys",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(Workspace::new()))
        }),
    )
}
