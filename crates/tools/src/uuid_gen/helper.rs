use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UuidVersion {
    One,
    Four,
    Seven,
}

impl UuidVersion {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "One" => Some(Self::One),
            "Four" => Some(Self::Four),
            "Seven" => Some(Self::Seven),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::One => "One",
            Self::Four => "Four",
            Self::Seven => "Seven",
        }
    }

    pub fn nibble(self) -> char {
        match self {
            Self::One => '1',
            Self::Four => '4',
            Self::Seven => '7',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UuidOptions {
    pub version: UuidVersion,
    pub hyphens: bool,
    pub uppercase: bool,
    pub count: usize,
}

impl Default for UuidOptions {
    fn default() -> Self {
        Self {
            version: UuidVersion::Four,
            hyphens: true,
            uppercase: false,
            count: 1,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UuidError {
    #[error("数量必须为正")]
    InvalidCount,
}

pub fn generate_uuid(options: &UuidOptions) -> Result<String, UuidError> {
    if options.count == 0 {
        return Err(UuidError::InvalidCount);
    }
    let mut lines = Vec::with_capacity(options.count);
    for _ in 0..options.count {
        lines.push(format_uuid(new_uuid(options.version), options));
    }
    Ok(lines.join("\n"))
}

fn new_uuid(version: UuidVersion) -> Uuid {
    match version {
        UuidVersion::One => {
            let mut node = [0u8; 6];
            getrandom_fill(&mut node);
            node[0] |= 0x01;
            Uuid::now_v1(&node)
        }
        UuidVersion::Four => Uuid::new_v4(),
        UuidVersion::Seven => Uuid::now_v7(),
    }
}

fn getrandom_fill(buf: &mut [u8]) {
    use rand::RngCore;
    rand::thread_rng().fill_bytes(buf);
}

fn format_uuid(uuid: Uuid, options: &UuidOptions) -> String {
    let text = if options.hyphens {
        uuid.hyphenated().to_string()
    } else {
        uuid.simple().to_string()
    };
    if options.uppercase {
        text.to_ascii_uppercase()
    } else {
        text
    }
}

pub fn version_nibble(text: &str) -> Option<char> {
    let compact: String = text.chars().filter(|c| *c != '-').collect();
    compact.chars().nth(12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_with_hyphens_matches_rfc_shape() {
        let got = generate_uuid(&UuidOptions::default()).unwrap();
        let re = regex::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        )
        .unwrap();
        assert!(re.is_match(&got), "{got}");
        assert_eq!(version_nibble(&got), Some('4'));
    }

    #[test]
    fn no_hyphen_length_32() {
        let got = generate_uuid(&UuidOptions {
            hyphens: false,
            ..UuidOptions::default()
        })
        .unwrap();
        assert_eq!(got.len(), 32);
        assert!(!got.contains('-'));
        assert!(got.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(got.as_bytes()[12], b'4');
    }

    #[test]
    fn uppercase_letters_are_a_to_f() {
        let got = generate_uuid(&UuidOptions {
            uppercase: true,
            ..UuidOptions::default()
        })
        .unwrap();
        assert!(got
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .all(|c| ('A'..='F').contains(&c)));
        assert!(!got.chars().any(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn v7_version_nibble_is_7() {
        let got = generate_uuid(&UuidOptions {
            version: UuidVersion::Seven,
            ..UuidOptions::default()
        })
        .unwrap();
        assert_eq!(version_nibble(&got), Some('7'));
        let parts: Vec<&str> = got.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert!(parts[2].starts_with('7'));
    }

    #[test]
    fn v1_version_nibble_is_1() {
        let got = generate_uuid(&UuidOptions {
            version: UuidVersion::One,
            ..UuidOptions::default()
        })
        .unwrap();
        assert_eq!(version_nibble(&got), Some('1'));
    }

    #[test]
    fn batch_count() {
        let got = generate_uuid(&UuidOptions {
            count: 3,
            ..UuidOptions::default()
        })
        .unwrap();
        assert_eq!(got.lines().count(), 3);
    }

    #[test]
    fn zero_count_is_err() {
        let err = generate_uuid(&UuidOptions {
            count: 0,
            ..UuidOptions::default()
        })
        .unwrap_err();
        assert_eq!(err, UuidError::InvalidCount);
    }
}
