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

#[derive(Clone, Copy)]
struct ArraySlice {
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>,
}

/// Evaluate `path` against `json`. Returns a pretty-printed JSON array of matches.
pub fn eval_jsonpath(json: &str, path: &str) -> Result<String, JsonPathError> {
    if path.trim().is_empty() {
        return Err(JsonPathError::InvalidPath);
    }
    let value: Value = serde_json::from_str(json).map_err(|_| JsonPathError::InvalidJson)?;
    // jsonpath-rust 0.7 的 step 为 unsigned，负步长/部分负起止需按 RFC 9535 自行切片。
    let found = match trailing_negative_slice(path) {
        Some((prefix, slice)) => {
            let prefix = if prefix.is_empty() { "$" } else { prefix };
            slice_nodes(query_nodes(&value, prefix)?, slice)
        }
        None => query_nodes(&value, path)?,
    };
    serde_json::to_string_pretty(&found).map_err(|_| JsonPathError::InvalidJson)
}

fn query_nodes(value: &Value, path: &str) -> Result<Value, JsonPathError> {
    let compiled = JsonPath::<Value>::try_from(path).map_err(|_| JsonPathError::InvalidPath)?;
    let found = compiled.find(value);
    if found.is_null() {
        Ok(Value::Array(Vec::new()))
    } else {
        Ok(found)
    }
}

fn trailing_negative_slice(path: &str) -> Option<(&str, ArraySlice)> {
    let path = path.trim();
    let rest = path.strip_suffix(']')?;
    let open = rest.rfind('[')?;
    let inner = rest[open + 1..].trim();
    if inner.contains(['?', '\'', '"', '*', ',', '[', ']']) {
        return None;
    }
    let parts: Vec<&str> = inner.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let parse_idx = |raw: &str| -> Option<Option<i64>> {
        let raw = raw.trim();
        if raw.is_empty() {
            Some(None)
        } else {
            Some(Some(raw.parse().ok()?))
        }
    };
    let start = parse_idx(parts[0])?;
    let end = parse_idx(parts[1])?;
    let step = if parts.len() == 3 {
        parse_idx(parts[2])?
    } else {
        None
    };
    let negative = matches!(start, Some(v) if v < 0)
        || matches!(end, Some(v) if v < 0)
        || matches!(step, Some(v) if v < 0);
    if !negative {
        return None;
    }
    Some((path[..open].trim(), ArraySlice { start, end, step }))
}

fn slice_nodes(nodes: Value, slice: ArraySlice) -> Value {
    let mut selected = Vec::new();
    if let Value::Array(matches) = nodes {
        for item in matches {
            if let Value::Array(arr) = item {
                selected.extend(rfc9535_slice(&arr, slice));
            }
        }
    }
    Value::Array(selected)
}

fn rfc9535_slice(arr: &[Value], slice: ArraySlice) -> Vec<Value> {
    let len = i64::try_from(arr.len()).unwrap_or(i64::MAX);
    let step = slice.step.unwrap_or(1);
    if step == 0 {
        return Vec::new();
    }
    let start = slice
        .start
        .unwrap_or(if step >= 0 { 0 } else { len.saturating_sub(1) });
    let end = slice.end.unwrap_or(if step >= 0 {
        len
    } else {
        len.saturating_neg().saturating_sub(1)
    });
    let n_start = normalize_index(start, len);
    let n_end = normalize_index(end, len);
    let (lower, upper) = if step >= 0 {
        (n_start.clamp(0, len), n_end.clamp(0, len))
    } else {
        let last = if len == 0 { -1 } else { len - 1 };
        (n_end.clamp(-1, last), n_start.clamp(-1, last))
    };
    let mut out = Vec::new();
    if step > 0 {
        let mut i = lower;
        while i < upper {
            push_index(&mut out, arr, i);
            i = match i.checked_add(step) {
                Some(next) => next,
                None => break,
            };
        }
    } else {
        let mut i = upper;
        while lower < i {
            push_index(&mut out, arr, i);
            i = match i.checked_add(step) {
                Some(next) => next,
                None => break,
            };
        }
    }
    out
}

fn normalize_index(i: i64, len: i64) -> i64 {
    if i >= 0 {
        i
    } else {
        len.saturating_add(i)
    }
}

fn push_index(out: &mut Vec<Value>, arr: &[Value], i: i64) {
    if let Ok(idx) = usize::try_from(i) {
        if let Some(v) = arr.get(idx) {
            out.push(v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(json: &str, path: &str) -> Value {
        serde_json::from_str(&eval_jsonpath(json, path).unwrap()).unwrap()
    }

    #[test]
    fn reverse_slice_preserves_order() {
        assert_eq!(parsed("[0,1,2]", "$[::-1]"), serde_json::json!([2, 1, 0]));
    }

    #[test]
    fn negative_start_last_element() {
        assert_eq!(parsed("[0,1,2]", "$[-1:]"), serde_json::json!([2]));
    }

    #[test]
    fn start_with_negative_step() {
        assert_eq!(parsed("[0,1,2]", "$[1::-1]"), serde_json::json!([1, 0]));
    }

    #[test]
    fn nested_reverse_slice() {
        assert_eq!(
            parsed(r#"{"items":[0,1,2]}"#, "$.items[::-1]"),
            serde_json::json!([2, 1, 0])
        );
    }

    #[test]
    fn positive_slice_and_filter_unchanged() {
        assert_eq!(parsed("[0,1,2]", "$[0:2]"), serde_json::json!([0, 1]));
        let got = eval_jsonpath(r#"[{"n":1},{"n":2}]"#, "$[?(@.n == 2)]").unwrap();
        assert!(got.contains('2'), "expected match 2, got {got}");
    }

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
