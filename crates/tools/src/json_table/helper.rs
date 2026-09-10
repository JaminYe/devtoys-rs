use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableFormat {
    #[default]
    Csv,
    Tsv,
    Fsv,
}

impl TableFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "CSV" | "Csv" => Some(Self::Csv),
            "TSV" | "Tsv" => Some(Self::Tsv),
            "FSV" | "Fsv" => Some(Self::Fsv),
            _ => None,
        }
    }

    pub fn delimiter(self) -> char {
        match self {
            Self::Csv => ',',
            Self::Tsv => '\t',
            Self::Fsv => ';',
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Fsv => "FSV",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsonTableError {
    #[error("非法 JSON")]
    InvalidJson,
    #[error("输入必须是对象数组")]
    NotObjectArray,
    #[error("空数组无法成表")]
    EmptyArray,
    #[error("没有可导出的列")]
    NoColumns,
}

pub fn json_to_table(input: &str, format: TableFormat) -> Result<String, JsonTableError> {
    let value: Value = serde_json::from_str(input).map_err(|_| JsonTableError::InvalidJson)?;
    let array = value.as_array().ok_or(JsonTableError::NotObjectArray)?;
    if array.is_empty() {
        return Err(JsonTableError::EmptyArray);
    }
    if !array.iter().all(Value::is_object) {
        return Err(JsonTableError::NotObjectArray);
    }

    let flattened: Vec<Map<String, Value>> = array
        .iter()
        .map(|item| flatten_object(item.as_object().expect("checked object array")))
        .collect();

    let mut headers: Vec<String> = Vec::new();
    for obj in &flattened {
        for key in obj.keys() {
            if !headers.iter().any(|h| h == key) {
                headers.push(key.clone());
            }
        }
    }
    if headers.is_empty() {
        return Err(JsonTableError::NoColumns);
    }

    let delim = format.delimiter();
    let mut lines = Vec::with_capacity(flattened.len() + 1);
    lines.push(join_fields(headers.iter().map(|h| h.as_str()), delim));
    for obj in &flattened {
        let cells: Vec<String> = headers
            .iter()
            .map(|key| match obj.get(key) {
                Some(value) => cell_value(value),
                None => String::new(),
            })
            .collect();
        lines.push(join_fields(cells.iter().map(String::as_str), delim));
    }
    Ok(lines.join("\n"))
}

/// Nested objects become `parent_child` keys; nested arrays are dropped.
fn flatten_object(obj: &Map<String, Value>) -> Map<String, Value> {
    let mut flattened = Map::new();
    for (key, value) in obj {
        match value {
            Value::Object(nested) => {
                for (child_key, child_value) in flatten_object(nested) {
                    flattened.insert(format!("{key}_{child_key}"), child_value);
                }
            }
            Value::Array(_) => {}
            other => {
                flattened.insert(key.clone(), other.clone());
            }
        }
    }
    flattened
}

fn cell_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn join_fields<'a>(fields: impl Iterator<Item = &'a str>, delim: char) -> String {
    fields
        .map(|field| escape_field(field, delim))
        .collect::<Vec<_>>()
        .join(&delim.to_string())
}

fn escape_field(field: &str, delim: char) -> String {
    if field.contains(delim) || field.contains('"') || field.contains('\n') || field.contains('\r')
    {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_objects_to_csv_header_and_rows() {
        let got = json_to_table(
            r#"[{"name":"Ada","id":1},{"name":"Bob","id":2}]"#,
            TableFormat::Csv,
        )
        .unwrap();
        assert_eq!(got, "name,id\nAda,1\nBob,2");
    }

    #[test]
    fn non_object_array_is_err() {
        let err = json_to_table("[1,2]", TableFormat::Csv).unwrap_err();
        assert_eq!(err, JsonTableError::NotObjectArray);
        assert!(!err.to_string().contains("[1,2]"));
        assert!(!err.to_string().contains('1'));
    }

    #[test]
    fn empty_array_is_err() {
        let err = json_to_table("[]", TableFormat::Csv).unwrap_err();
        assert_eq!(err, JsonTableError::EmptyArray);
        let message = err.to_string();
        assert_eq!(message, "空数组无法成表");
        assert!(!message.contains('['));
    }

    #[test]
    fn csv_quotes_delimiter_in_field() {
        let got = json_to_table(r#"[{"a":"x,y"}]"#, TableFormat::Csv).unwrap();
        assert_eq!(got, "a\n\"x,y\"");
    }

    #[test]
    fn nested_object_flattens_to_parent_child_column() {
        let got = json_to_table(r#"[{"a":{"b":1}}]"#, TableFormat::Csv).unwrap();
        assert_eq!(got, "a_b\n1");
    }

    #[test]
    fn multi_level_nested_object_joins_ancestor_chain() {
        let got = json_to_table(r#"[{"a":{"b":{"c":2}}}]"#, TableFormat::Csv).unwrap();
        assert_eq!(got, "a_b_c\n2");
    }

    #[test]
    fn mixed_flat_and_nested_fields_keep_and_flatten() {
        let got = json_to_table(r#"[{"x":1,"a":{"b":2}}]"#, TableFormat::Csv).unwrap();
        assert_eq!(got, "x,a_b\n1,2");
    }

    #[test]
    fn nested_array_is_stripped_not_expanded() {
        let got = json_to_table(r#"[{"a":[1,2],"b":3}]"#, TableFormat::Csv).unwrap();
        assert_eq!(got, "b\n3");
        assert!(!got.contains("a"));
    }
}
