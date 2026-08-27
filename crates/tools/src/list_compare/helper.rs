use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListMode {
    #[default]
    AInterB,
    AUnionB,
    AOnly,
    BOnly,
}

impl ListMode {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "AInterB" => Self::AInterB,
            "AUnionB" => Self::AUnionB,
            "AOnly" => Self::AOnly,
            "BOnly" => Self::BOnly,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AInterB => "AInterB",
            Self::AUnionB => "AUnionB",
            Self::AOnly => "AOnly",
            Self::BOnly => "BOnly",
        }
    }
}

fn parse_list(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn key(item: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        item.to_string()
    } else {
        item.to_lowercase()
    }
}

fn unique(items: &[String], case_sensitive: bool) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(key(item, case_sensitive)) {
            out.push(item.clone());
        }
    }
    out
}

pub fn compare_lists(a: &str, b: &str, mode: ListMode, case_sensitive: bool) -> String {
    let list_a = unique(&parse_list(a), case_sensitive);
    let list_b = unique(&parse_list(b), case_sensitive);
    let keys_a: HashSet<String> = list_a.iter().map(|s| key(s, case_sensitive)).collect();
    let keys_b: HashSet<String> = list_b.iter().map(|s| key(s, case_sensitive)).collect();

    let result: Vec<String> = match mode {
        ListMode::AInterB => list_a
            .into_iter()
            .filter(|s| keys_b.contains(&key(s, case_sensitive)))
            .collect(),
        ListMode::AUnionB => {
            let mut out = list_a;
            for item in list_b {
                if !keys_a.contains(&key(&item, case_sensitive)) {
                    out.push(item);
                }
            }
            out
        }
        ListMode::AOnly => list_a
            .into_iter()
            .filter(|s| !keys_b.contains(&key(s, case_sensitive)))
            .collect(),
        ListMode::BOnly => list_b
            .into_iter()
            .filter(|s| !keys_a.contains(&key(s, case_sensitive)))
            .collect(),
    };
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_union_aonly() {
        let a = "a\nb\n";
        let b = "b\nc";
        assert_eq!(compare_lists(a, b, ListMode::AInterB, true), "b");
        assert_eq!(compare_lists(a, b, ListMode::AUnionB, true), "a\nb\nc");
        assert_eq!(compare_lists(a, b, ListMode::AOnly, true), "a");
        assert_eq!(compare_lists(a, b, ListMode::BOnly, true), "c");
    }

    #[test]
    fn case_insensitive_intersect_keeps_a_original() {
        assert_eq!(compare_lists("A", "a", ListMode::AInterB, false), "A");
        assert_eq!(compare_lists("A", "a", ListMode::AInterB, true), "");
    }
}
