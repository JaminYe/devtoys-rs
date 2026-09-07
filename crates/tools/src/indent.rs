/// Shared indentation for JSON / YAML / SQL / XML formatters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Indentation {
    #[default]
    TwoSpaces,
    FourSpaces,
    OneTab,
    Minified,
}

impl Indentation {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "TwoSpaces" => Some(Self::TwoSpaces),
            "FourSpaces" => Some(Self::FourSpaces),
            "OneTab" => Some(Self::OneTab),
            "Minified" => Some(Self::Minified),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TwoSpaces => "TwoSpaces",
            Self::FourSpaces => "FourSpaces",
            Self::OneTab => "OneTab",
            Self::Minified => "Minified",
        }
    }

    /// Indent bytes for pretty printers. `None` means minify.
    pub fn as_bytes(self) -> Option<&'static [u8]> {
        match self {
            Self::TwoSpaces => Some(b"  "),
            Self::FourSpaces => Some(b"    "),
            Self::OneTab => Some(b"\t"),
            Self::Minified => None,
        }
    }
}
