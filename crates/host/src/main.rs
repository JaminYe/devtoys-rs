#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod system_fonts;
mod theme;
mod widgets;

use app::Workspace;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DevToys")
            .with_app_id("devtoys")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([760.0, 520.0]),
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
