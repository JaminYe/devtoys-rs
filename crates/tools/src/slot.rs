use gpui::{AnyView, App, AppContext, Context, Entity, Render, Window};

/// Smart Detection paste target. Views implement this; the host never names a tool.
pub trait ReceivesData: Sized {
    fn on_data_received(&mut self, payload: &str, window: &mut Window, cx: &mut Context<Self>);
}

/// Opaque GUI session for one open tool. Host stores these by tool id.
pub struct ToolHandle {
    pub view: AnyView,
    paste: Box<dyn FnMut(&str, &mut Window, &mut App)>,
}

impl ToolHandle {
    pub fn open<V>(
        window: &mut Window,
        cx: &mut App,
        build: impl FnOnce(&mut Window, &mut Context<V>) -> V,
    ) -> Self
    where
        V: Render + ReceivesData + 'static,
    {
        let entity: Entity<V> = cx.new(|cx| build(window, cx));
        let paste_entity = entity.clone();
        Self {
            view: entity.into(),
            paste: Box::new(move |payload, window, cx| {
                paste_entity.update(cx, |view, cx| {
                    view.on_data_received(payload, window, cx);
                });
            }),
        }
    }

    pub fn on_data_received(&mut self, payload: &str, window: &mut Window, cx: &mut App) {
        (self.paste)(payload, window, cx);
    }
}
