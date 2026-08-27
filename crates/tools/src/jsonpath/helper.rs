use jsonpath_rust::JsonPath;
use serde_json::Value;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsonPathError {
    #[error("非法 JSON")]
    InvalidJson,
    #[error("非法 JSONPath")]
    InvalidPath,
}

/// Common JSONPath tokens for the cheat sheet.
pub const CHEAT_SHEET: &[(&str, &str)] = &[
    ("$", "根对象"),
    ("@", "当前对象"),
    (".property", "点号取子属性"),
    ("['property']", "方括号取子属性"),
    ("..property", "递归下降"),
    ("*", "通配符"),
    ("[n]", "下标"),
    ("[n1,n2]", "联合下标"),
    ("[start:end:step]", "切片"),
    ("?(expression)", "过滤表达式"),
];

/// Evaluate `path` against `json`. Returns a pretty-printed JSON array of matches.
pub fn eval_jsonpath(json: &str, path: &str) -> Result<String, JsonPathError> {
    if path.trim().is_empty() {
        return Err(JsonPathError::InvalidPath);
    }
    let value: Value = serde_json::from_str(json).map_err(|_| JsonPathError::InvalidJson)?;
    let compiled = JsonPath::<Value>::try_from(path).map_err(|_| JsonPathError::InvalidPath)?;
    let found = compiled.find(&value);
    let found = if found.is_null() {
        Value::Array(Vec::new())
    } else {
        found
    };
    serde_json::to_string_pretty(&found).map_err(|_| JsonPathError::InvalidJson)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_field_contains_one() {
        let got = eval_jsonpath(r#"{"a":{"b":1}}"#, "$.a.b").unwrap();
        assert!(got.contains('1'), "expected match 1, got {got}");
    }

    #[test]
    fn bad_json_is_err_without_input() {
        let err = eval_jsonpath("{", "$.a").unwrap_err();
        assert_eq!(err, JsonPathError::InvalidJson);
        let message = err.to_string();
        assert!(!message.contains('{'), "error must not include user input");
    }

    #[test]
    fn bad_path_is_err_without_input() {
        let err = eval_jsonpath(r#"{"a":1}"#, "[").unwrap_err();
        assert_eq!(err, JsonPathError::InvalidPath);
        let message = err.to_string();
        assert!(!message.contains('['), "error must not include user input");
    }

    #[test]
    fn cheat_sheet_lists_root() {
        assert!(CHEAT_SHEET.iter().any(|(s, _)| *s == "$"));
    }
}
