#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod workspace;

use gpui::{
    px, size, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use gpui_component::Root;

use assets::Assets;
use workspace::Workspace;

fn main() {
    let app = Application::new().with_assets(Assets);
    app.run(|cx| {
        gpui_component::init(cx);
        let bounds = Bounds::centered(None, size(px(1120.), px(720.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("DevToys".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("DevToys");
                let view = cx.new(|cx| Workspace::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .unwrap();
    });
}
