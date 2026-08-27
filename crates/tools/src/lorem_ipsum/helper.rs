#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corpus {
    LoremIpsum,
    ChildHarold,
    Decameron,
    Faust,
    InDerFremde,
    LeBateauIvre,
    LeMasque,
    NagyonFaj,
    Omagyar,
    RobinsonoKruso,
    TheRaven,
    TierrayLuna,
}

impl Corpus {
    pub const ALL: [Corpus; 12] = [
        Corpus::LoremIpsum,
        Corpus::ChildHarold,
        Corpus::Decameron,
        Corpus::Faust,
        Corpus::InDerFremde,
        Corpus::LeBateauIvre,
        Corpus::LeMasque,
        Corpus::NagyonFaj,
        Corpus::Omagyar,
        Corpus::RobinsonoKruso,
        Corpus::TheRaven,
        Corpus::TierrayLuna,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "LoremIpsum" => Some(Self::LoremIpsum),
            "ChildHarold" => Some(Self::ChildHarold),
            "Decameron" => Some(Self::Decameron),
            "Faust" => Some(Self::Faust),
            "InDerFremde" => Some(Self::InDerFremde),
            "LeBateauIvre" => Some(Self::LeBateauIvre),
            "LeMasque" => Some(Self::LeMasque),
            "NagyonFaj" => Some(Self::NagyonFaj),
            "Omagyar" => Some(Self::Omagyar),
            "RobinsonoKruso" => Some(Self::RobinsonoKruso),
            "TheRaven" => Some(Self::TheRaven),
            "TierrayLuna" => Some(Self::TierrayLuna),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoremIpsum => "LoremIpsum",
            Self::ChildHarold => "ChildHarold",
            Self::Decameron => "Decameron",
            Self::Faust => "Faust",
            Self::InDerFremde => "InDerFremde",
            Self::LeBateauIvre => "LeBateauIvre",
            Self::LeMasque => "LeMasque",
            Self::NagyonFaj => "NagyonFaj",
            Self::Omagyar => "Omagyar",
            Self::RobinsonoKruso => "RobinsonoKruso",
            Self::TheRaven => "TheRaven",
            Self::TierrayLuna => "TierrayLuna",
        }
    }

    pub fn excerpt(self) -> &'static str {
        match self {
            Self::LoremIpsum => {
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris."
            }
            Self::ChildHarold => {
                "Oh, thou Parnassus, whom I now survey, not in the frenzy of a dreamer's eye. Once more upon the waters, yet once more."
            }
            Self::Decameron => {
                "A kindly thing it is to have compassion on the afflicted. In the year of our Lord 1348, a deadly pestilence came to Florence."
            }
            Self::Faust => {
                "Habe nun, ach! Philosophie, Juristerei und Medizin, und leider auch Theologie durchaus studiert, mit heissem Bemuehn."
            }
            Self::InDerFremde => {
                "Ich hatte einst ein schoenes Vaterland. Der Eichenbaum wuchs dort so hoch, die Veilchen nickten sanft."
            }
            Self::LeBateauIvre => {
                "Comme je descendais des Fleuves impassibles, je me sentis fleurer. Plus doux que la chair des pommes."
            }
            Self::LeMasque => {
                "Oublions le caveau, lorgnons la mascarade. Le monde est un theatre, et la vie un ballet."
            }
            Self::NagyonFaj => {
                "Nagyon faj a szivem, de nem sirhatok. Az ejszaka csendes, a hold vilagit a vizen."
            }
            Self::Omagyar => {
                "Latod uram, istenem, mire jutottam. Regi magyar szo, regi magyar dal a mezok felett."
            }
            Self::RobinsonoKruso => {
                "Mi naskighis en la jaro 1632, en la urbo Jorko. Mia patro estis kura commerciisto el Bremen."
            }
            Self::TheRaven => {
                "Once upon a midnight dreary, while I pondered, weak and weary. Over many a quaint and curious volume of forgotten lore."
            }
            Self::TierrayLuna => {
                "The woods are lovely, dark and deep. But I have promises to keep, and miles to go before I sleep."
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoremUnit {
    Paragraphs,
    Sentences,
    Words,
    Characters,
}

impl LoremUnit {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Paragraphs" => Some(Self::Paragraphs),
            "Sentences" => Some(Self::Sentences),
            "Words" => Some(Self::Words),
            "Characters" => Some(Self::Characters),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paragraphs => "Paragraphs",
            Self::Sentences => "Sentences",
            Self::Words => "Words",
            Self::Characters => "Characters",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LoremError {
    #[error("长度必须为正")]
    InvalidLength,
}

/// Deterministic placeholder text: cycle the embedded excerpt. No RNG.
pub fn generate_lorem(corpus: Corpus, unit: LoremUnit, length: usize) -> Result<String, LoremError> {
    if length == 0 {
        return Err(LoremError::InvalidLength);
    }
    let excerpt = corpus.excerpt();
    Ok(match unit {
        LoremUnit::Characters => cycle_chars(excerpt, length),
        LoremUnit::Words => cycle_join(&words(excerpt), length, " "),
        LoremUnit::Sentences => cycle_join(&sentences(excerpt), length, " "),
        LoremUnit::Paragraphs => cycle_join(&paragraphs(excerpt), length, "\n\n"),
    })
}

fn words(excerpt: &str) -> Vec<&str> {
    excerpt.split_whitespace().collect()
}

fn sentences(excerpt: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in excerpt.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = cur.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            cur.clear();
        }
    }
    let trimmed = cur.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    if out.is_empty() {
        out.push(excerpt.trim().to_string());
    }
    out
}

fn paragraphs(excerpt: &str) -> Vec<&str> {
    let parts: Vec<&str> = excerpt
        .split("\n\n")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        vec![excerpt.trim()]
    } else {
        parts
    }
}

fn cycle_chars(excerpt: &str, length: usize) -> String {
    let chars: Vec<char> = excerpt.chars().collect();
    (0..length).map(|i| chars[i % chars.len()]).collect()
}

fn cycle_join(items: &[impl AsRef<str>], length: usize, sep: &str) -> String {
    (0..length)
        .map(|i| items[i % items.len()].as_ref().to_string())
        .collect::<Vec<_>>()
        .join(sep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_corpora() {
        assert_eq!(Corpus::ALL.len(), 12);
        for corpus in Corpus::ALL {
            assert!(!corpus.excerpt().is_empty());
            assert_eq!(Corpus::parse(corpus.as_str()), Some(corpus));
        }
    }

    #[test]
    fn lorem_words_length_3() {
        let got = generate_lorem(Corpus::LoremIpsum, LoremUnit::Words, 3).unwrap();
        let words: Vec<&str> = got.split_whitespace().collect();
        assert_eq!(words.len(), 3);
        let corpus_words: Vec<&str> = Corpus::LoremIpsum.excerpt().split_whitespace().collect();
        assert_eq!(words, &corpus_words[..3]);
        assert_eq!(got, "Lorem ipsum dolor");
    }

    #[test]
    fn lorem_characters_length_10() {
        let got = generate_lorem(Corpus::LoremIpsum, LoremUnit::Characters, 10).unwrap();
        assert_eq!(got.chars().count(), 10);
        let prefix: String = Corpus::LoremIpsum.excerpt().chars().take(10).collect();
        assert_eq!(got, prefix);
    }

    #[test]
    fn same_inputs_are_deterministic() {
        let a = generate_lorem(Corpus::TheRaven, LoremUnit::Sentences, 2).unwrap();
        let b = generate_lorem(Corpus::TheRaven, LoremUnit::Sentences, 2).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.split(". ").count().max(a.matches('.').count()), a.matches('.').count());
    }

    #[test]
    fn paragraphs_cycle_without_rand() {
        let got = generate_lorem(Corpus::Faust, LoremUnit::Paragraphs, 2).unwrap();
        let parts: Vec<&str> = got.split("\n\n").collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], parts[1]);
        assert_eq!(parts[0], Corpus::Faust.excerpt());
    }

    #[test]
    fn zero_length_is_err() {
        let err = generate_lorem(Corpus::LoremIpsum, LoremUnit::Words, 0).unwrap_err();
        assert_eq!(err, LoremError::InvalidLength);
    }
}
