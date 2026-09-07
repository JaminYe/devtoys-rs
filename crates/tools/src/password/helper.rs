use rand::Rng;

pub const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
pub const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
pub const DIGITS: &str = "0123456789";
/// Non-alphanumeric set used by DevToys. Excludes `/ < [ \` `{` `|`.
pub const SPECIAL: &str = "!\"#$%&')*+,-.:;=>?@]^_}~";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordOptions {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub special: bool,
    pub exclude: String,
    pub count: usize,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 30,
            uppercase: true,
            lowercase: true,
            digits: true,
            special: true,
            exclude: String::new(),
            count: 1,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PasswordError {
    #[error("字符集为空")]
    EmptyCharset,
}

pub fn charset_for(options: &PasswordOptions) -> String {
    let excluded: Vec<char> = options.exclude.chars().collect();
    let mut out = String::new();
    if options.uppercase {
        push_filtered(&mut out, UPPERCASE, &excluded);
    }
    if options.lowercase {
        push_filtered(&mut out, LOWERCASE, &excluded);
    }
    if options.digits {
        push_filtered(&mut out, DIGITS, &excluded);
    }
    if options.special {
        push_filtered(&mut out, SPECIAL, &excluded);
    }
    out
}

fn push_filtered(out: &mut String, source: &str, excluded: &[char]) {
    for ch in source.chars() {
        if !excluded.contains(&ch) {
            out.push(ch);
        }
    }
}

pub fn generate_password(
    options: &PasswordOptions,
    rng: &mut impl Rng,
) -> Result<String, PasswordError> {
    let charset: Vec<char> = charset_for(options).chars().collect();
    if charset.is_empty() {
        return Err(PasswordError::EmptyCharset);
    }
    let count = options.count.max(1);
    let mut lines = Vec::with_capacity(count);
    for _ in 0..count {
        let mut password = String::with_capacity(options.length);
        for _ in 0..options.length {
            let idx = rng.gen_range(0..charset.len());
            password.push(charset[idx]);
        }
        lines.push(password);
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn digits_only(length: usize) -> PasswordOptions {
        PasswordOptions {
            length,
            uppercase: false,
            lowercase: false,
            digits: true,
            special: false,
            exclude: String::new(),
            count: 1,
        }
    }

    #[test]
    fn digits_only_are_all_digits() {
        let mut rng = StdRng::seed_from_u64(1);
        let got = generate_password(&digits_only(8), &mut rng).unwrap();
        assert_eq!(got.chars().count(), 8);
        assert!(got.chars().all(|c| c.is_ascii_digit()));
        assert!(got.chars().all(|c| DIGITS.contains(c)));
    }

    #[test]
    fn exclude_a_from_lowercase() {
        let mut rng = StdRng::seed_from_u64(1);
        let options = PasswordOptions {
            length: 24,
            uppercase: false,
            lowercase: true,
            digits: false,
            special: false,
            exclude: "a".into(),
            count: 1,
        };
        let got = generate_password(&options, &mut rng).unwrap();
        assert_eq!(got.chars().count(), 24);
        assert!(!got.contains('a'));
        assert!(got.chars().all(|c| c.is_ascii_lowercase()));
        assert!(got.chars().all(|c| LOWERCASE.contains(c) && c != 'a'));
    }

    #[test]
    fn empty_charset_is_err() {
        let mut rng = StdRng::seed_from_u64(1);
        let all_off = PasswordOptions {
            length: 8,
            uppercase: false,
            lowercase: false,
            digits: false,
            special: false,
            exclude: String::new(),
            count: 1,
        };
        assert_eq!(
            generate_password(&all_off, &mut rng).unwrap_err(),
            PasswordError::EmptyCharset
        );

        let excluded_all = PasswordOptions {
            length: 8,
            uppercase: false,
            lowercase: true,
            digits: false,
            special: false,
            exclude: LOWERCASE.into(),
            count: 1,
        };
        assert_eq!(
            generate_password(&excluded_all, &mut rng).unwrap_err(),
            PasswordError::EmptyCharset
        );
        assert_eq!(PasswordError::EmptyCharset.to_string(), "字符集为空");
    }

    #[test]
    fn seeded_rng_is_stable_without_golden_secret() {
        let options = digits_only(8);
        let a = generate_password(&options, &mut StdRng::seed_from_u64(1)).unwrap();
        let b = generate_password(&options, &mut StdRng::seed_from_u64(1)).unwrap();
        let c = generate_password(&options, &mut StdRng::seed_from_u64(2)).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.chars().all(|ch| ch.is_ascii_digit()));
    }

    #[test]
    fn batch_count_emits_lines() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut options = digits_only(4);
        options.count = 3;
        let got = generate_password(&options, &mut rng).unwrap();
        let lines: Vec<&str> = got.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines
            .iter()
            .all(|line| line.chars().all(|c| c.is_ascii_digit())));
    }
}
