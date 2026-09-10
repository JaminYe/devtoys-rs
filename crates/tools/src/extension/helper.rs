//! Manifest extension transform: echo or uppercase. GUI and CLI share this.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transform {
    Echo,
    #[default]
    Uppercase,
}

impl Transform {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "echo" => Some(Self::Echo),
            "uppercase" => Some(Self::Uppercase),
            _ => None,
        }
    }
}

pub fn apply_transform(input: &str, transform: Transform) -> String {
    match transform {
        Transform::Echo => input.to_string(),
        Transform::Uppercase => input.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_is_a_visible_transform() {
        assert_eq!(apply_transform("hello", Transform::Uppercase), "HELLO");
        assert_eq!(apply_transform("Straße", Transform::Uppercase), "STRASSE");
    }

    #[test]
    fn echo_preserves_input() {
        assert_eq!(apply_transform("Hello", Transform::Echo), "Hello");
        assert_eq!(apply_transform("", Transform::Echo), "");
    }

    #[test]
    fn parse_rejects_unknown_mode() {
        assert_eq!(Transform::parse("uppercase"), Some(Transform::Uppercase));
        assert_eq!(Transform::parse("echo"), Some(Transform::Echo));
        assert_eq!(Transform::parse("title"), None);
    }
}
