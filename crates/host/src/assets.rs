use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "icons/a-large-small.svg" => Some(include_bytes!("../assets/icons/a-large-small.svg")),
            "icons/bot.svg" => Some(include_bytes!("../assets/icons/bot.svg")),
            "icons/calendar.svg" => Some(include_bytes!("../assets/icons/calendar.svg")),
            "icons/case-sensitive.svg" => {
                Some(include_bytes!("../assets/icons/case-sensitive.svg"))
            }
            "icons/chevron-right.svg" => Some(include_bytes!("../assets/icons/chevron-right.svg")),
            "icons/close.svg" => Some(include_bytes!("../assets/icons/close.svg")),
            "icons/inbox.svg" => Some(include_bytes!("../assets/icons/inbox.svg")),
            "icons/inspector.svg" => Some(include_bytes!("../assets/icons/inspector.svg")),
            "icons/layout-dashboard.svg" => {
                Some(include_bytes!("../assets/icons/layout-dashboard.svg"))
            }
            "icons/palette.svg" => Some(include_bytes!("../assets/icons/palette.svg")),
            "icons/plus.svg" => Some(include_bytes!("../assets/icons/plus.svg")),
            "icons/replace.svg" => Some(include_bytes!("../assets/icons/replace.svg")),
            "icons/search.svg" => Some(include_bytes!("../assets/icons/search.svg")),
            "icons/settings.svg" => Some(include_bytes!("../assets/icons/settings.svg")),
            "icons/settings-2.svg" => Some(include_bytes!("../assets/icons/settings-2.svg")),
            "icons/square-terminal.svg" => {
                Some(include_bytes!("../assets/icons/square-terminal.svg"))
            }
            "icons/star.svg" => Some(include_bytes!("../assets/icons/star.svg")),
            "icons/star-off.svg" => Some(include_bytes!("../assets/icons/star-off.svg")),
            _ => None,
        };

        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
