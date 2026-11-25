use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use serde_yaml_ng::{Mapping, Value as YamlValue};

use super::{compare_json_values, split_key_path, Format, FrontmatterError};

/// Formatting options controlling YAML frontmatter serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlFormatOptions {
    /// Indent YAML sequence items (`- `) under their parent mapping key.
    pub indent_sequences: bool,
    /// Number of spaces per indentation level (must be >= 1; default: 2).
    pub yaml_indent: usize,
}

impl Default for YamlFormatOptions {
    fn default() -> Self {
        Self {
            indent_sequences: true,
            yaml_indent: 2,
        }
    }
}

/// Parse YAML frontmatter text into a `serde_yaml_ng::Mapping`.
///
/// Empty or comment-only blocks yield an empty `Mapping` without erroring.
pub fn parse_yaml(raw: &str) -> Result<Mapping, FrontmatterError> {
    if raw.trim().is_empty() {
        return Ok(Mapping::new());
    }

    let value: YamlValue = serde_yaml_ng::from_str(raw)?;
    match value {
        YamlValue::Mapping(mapping) => Ok(mapping),
        YamlValue::Null => Ok(Mapping::new()),
        _ => Err(FrontmatterError::InvalidRoot {
            format: Format::Yaml,
        }),
    }
}

/// Serialize a YAML `Mapping` to a YAML string (without `---` delimiters) using default options.
///
/// An empty mapping serializes to `""` so empty frontmatter blocks round-trip cleanly.
pub fn serialize_yaml(mapping: &Mapping) -> Result<String, FrontmatterError> {
    serialize_yaml_with_options(mapping, YamlFormatOptions::default())
}

/// Serialize a YAML `Mapping` with configurable sequence indentation and indent width.
pub fn serialize_yaml_with_options(
    mapping: &Mapping,
    options: YamlFormatOptions,
) -> Result<String, FrontmatterError> {
    if mapping.is_empty() {
        return Ok(String::new());
    }
    let normalized = YamlFormatOptions {
        indent_sequences: options.indent_sequences,
        yaml_indent: options.yaml_indent.max(1),
    };
    let mut out = String::new();
    emit_mapping(mapping, 0, false, normalized, &mut out)?;
    Ok(out)
}

fn push_spaces(out: &mut String, count: usize) {
    for _ in 0..count {
        out.push(' ');
    }
}

fn format_simple_key(key: &YamlValue) -> Result<Option<String>, FrontmatterError> {
    if matches!(key, YamlValue::Sequence(_) | YamlValue::Mapping(_)) {
        return Ok(None);
    }
    let mut probe = Mapping::with_capacity(1);
    probe.insert(key.clone(), YamlValue::Null);
    let rendered = serde_yaml_ng::to_string(&probe)?;
    if let Some(stripped) = rendered.strip_suffix(": null\n") {
        if !stripped.contains('\n') {
            return Ok(Some(stripped.to_string()));
        }
    }
    Ok(None)
}

fn rewrite_block_scalar_header(header: &str, rel_indent: usize) -> (String, Option<usize>) {
    let bytes = header.as_bytes();
    if bytes.is_empty() || (bytes[0] != b'|' && bytes[0] != b'>') {
        return (header.to_string(), None);
    }
    let rest = &header[1..];
    let digit_pos = rest.find(|c: char| c.is_ascii_digit());
    let Some(pos) = digit_pos else {
        return (header.to_string(), None);
    };
    let digit_end = rest[pos..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|off| pos + off)
        .unwrap_or(rest.len());
    if (1..=9).contains(&rel_indent) {
        let mut rebuilt = String::with_capacity(header.len());
        rebuilt.push(bytes[0] as char);
        rebuilt.push_str(&rest[..pos]);
        rebuilt.push((b'0' + rel_indent as u8) as char);
        rebuilt.push_str(&rest[digit_end..]);
        (rebuilt, Some(rel_indent))
    } else {
        let existing = rest[pos..digit_end].parse::<usize>().ok();
        (header.to_string(), existing)
    }
}

fn emit_scalar_value(
    value: &YamlValue,
    parent_indent: usize,
    content_indent: usize,
    out: &mut String,
) -> Result<(), FrontmatterError> {
    let mut probe = Mapping::with_capacity(1);
    probe.insert(YamlValue::String("k".to_string()), value.clone());
    let rendered = serde_yaml_ng::to_string(&probe)?;

    if let Some(rest) = rendered.strip_prefix("k: ") {
        let trimmed = rest.strip_suffix('\n').unwrap_or(rest);
        let mut lines = trimmed.split('\n');
        let first = lines.next().unwrap_or("");
        let remaining: Vec<&str> = lines.collect();
        if remaining.is_empty() {
            out.push_str(first);
            out.push('\n');
        } else {
            let rel = content_indent.saturating_sub(parent_indent);
            let (header, explicit_rel) = rewrite_block_scalar_header(first, rel);
            let target_indent = explicit_rel
                .map(|r| parent_indent + r)
                .unwrap_or(content_indent);
            out.push_str(&header);
            out.push('\n');
            for line in remaining {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    let body = line.strip_prefix("  ").unwrap_or(line);
                    push_spaces(out, target_indent);
                    out.push_str(body);
                    out.push('\n');
                }
            }
        }
        return Ok(());
    }

    if let Some(rest) = rendered.strip_prefix("k:\n") {
        out.push('\n');
        let trimmed = rest.strip_suffix('\n').unwrap_or(rest);
        for line in trimmed.split('\n') {
            if line.is_empty() {
                out.push('\n');
            } else {
                let body = line.strip_prefix("  ").unwrap_or(line);
                push_spaces(out, content_indent);
                out.push_str(body);
                out.push('\n');
            }
        }
        return Ok(());
    }

    out.push_str(&rendered);
    Ok(())
}

fn emit_mapping(
    mapping: &Mapping,
    indent: usize,
    first_line_indented: bool,
    options: YamlFormatOptions,
    out: &mut String,
) -> Result<(), FrontmatterError> {
    for (idx, (key, val)) in mapping.iter().enumerate() {
        let skip_indent = idx == 0 && first_line_indented;
        if let Some(key_str) = format_simple_key(key)? {
            if !skip_indent {
                push_spaces(out, indent);
            }
            out.push_str(&key_str);
            out.push(':');

            match val {
                YamlValue::Sequence(seq) if seq.is_empty() => {
                    out.push_str(" []\n");
                }
                YamlValue::Sequence(seq) => {
                    out.push('\n');
                    let seq_indent = if options.indent_sequences {
                        indent + options.yaml_indent
                    } else {
                        indent
                    };
                    emit_sequence(seq, seq_indent, false, options, out)?;
                }
                YamlValue::Mapping(child) if child.is_empty() => {
                    out.push_str(" {}\n");
                }
                YamlValue::Mapping(child) => {
                    out.push('\n');
                    emit_mapping(child, indent + options.yaml_indent, false, options, out)?;
                }
                YamlValue::Tagged(tagged) => match &tagged.value {
                    YamlValue::Sequence(seq) if !seq.is_empty() => {
                        out.push(' ');
                        out.push_str(&tagged.tag.to_string());
                        out.push('\n');
                        let seq_indent = if options.indent_sequences {
                            indent + options.yaml_indent
                        } else {
                            indent
                        };
                        emit_sequence(seq, seq_indent, false, options, out)?;
                    }
                    YamlValue::Mapping(child) if !child.is_empty() => {
                        out.push(' ');
                        out.push_str(&tagged.tag.to_string());
                        out.push('\n');
                        emit_mapping(child, indent + options.yaml_indent, false, options, out)?;
                    }
                    _ => {
                        out.push(' ');
                        emit_scalar_value(val, indent, indent + options.yaml_indent, out)?;
                    }
                },
                _ => {
                    out.push(' ');
                    emit_scalar_value(val, indent, indent + options.yaml_indent, out)?;
                }
            }
        } else {
            // Complex (sequence/mapping/multiline) YAML key: delegate single entry to serde_yaml_ng.
            let mut single = Mapping::with_capacity(1);
            single.insert(key.clone(), val.clone());
            let rendered = serde_yaml_ng::to_string(&single)?;
            let trimmed = rendered.strip_suffix('\n').unwrap_or(&rendered);
            for (line_idx, line) in trimmed.split('\n').enumerate() {
                if line.is_empty() {
                    out.push('\n');
                } else {
                    if !(skip_indent && line_idx == 0) {
                        push_spaces(out, indent);
                    }
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
    }
    Ok(())
}

fn emit_sequence(
    seq: &[YamlValue],
    indent: usize,
    first_line_indented: bool,
    options: YamlFormatOptions,
    out: &mut String,
) -> Result<(), FrontmatterError> {
    for (idx, item) in seq.iter().enumerate() {
        if !(idx == 0 && first_line_indented) {
            push_spaces(out, indent);
        }
        out.push('-');

        match item {
            YamlValue::Sequence(nested) if nested.is_empty() => {
                out.push_str(" []\n");
            }
            YamlValue::Sequence(nested) => {
                out.push(' ');
                emit_sequence(nested, indent + 2, true, options, out)?;
            }
            YamlValue::Mapping(map) if map.is_empty() => {
                out.push_str(" {}\n");
            }
            YamlValue::Mapping(map) => {
                out.push(' ');
                emit_mapping(map, indent + 2, true, options, out)?;
            }
            YamlValue::Tagged(tagged) => match &tagged.value {
                YamlValue::Sequence(nested) if !nested.is_empty() => {
                    out.push(' ');
                    out.push_str(&tagged.tag.to_string());
                    out.push('\n');
                    emit_sequence(nested, indent + 2, false, options, out)?;
                }
                YamlValue::Mapping(map) if !map.is_empty() => {
                    out.push(' ');
                    out.push_str(&tagged.tag.to_string());
                    out.push('\n');
                    emit_mapping(map, indent + 2, false, options, out)?;
                }
                _ => {
                    out.push(' ');
                    emit_scalar_value(item, indent, indent + 2, out)?;
                }
            },
            _ => {
                out.push(' ');
                emit_scalar_value(item, indent, indent + 2, out)?;
            }
        }
    }
    Ok(())
}

/// Recursively sort all YAML mapping keys alphabetically.
pub fn sort_keys(mapping: &mut Mapping) {
    let mut entries: Vec<(YamlValue, YamlValue)> = mapping
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (_, v) in &mut entries {
        sort_value_keys(v);
    }
    entries.sort_by_key(|(k1, _)| yaml_key_to_string(k1));

    mapping.clear();
    for (k, v) in entries {
        mapping.insert(k, v);
    }
}

fn sort_value_keys(value: &mut YamlValue) {
    match value {
        YamlValue::Mapping(map) => sort_keys(map),
        YamlValue::Sequence(seq) => {
            for item in seq {
                sort_value_keys(item);
            }
        }
        YamlValue::Tagged(tagged) => sort_value_keys(&mut tagged.value),
        _ => {}
    }
}

/// Sort and deduplicate a YAML sequence at `path`.
///
/// Missing keys or intermediate mappings are treated as no-ops and return `Ok(false)`.
pub fn sort_array(mapping: &mut Mapping, path: &str) -> Result<bool, FrontmatterError> {
    let segments = split_key_path(path)?;
    let mut current = mapping;

    for seg in &segments[..segments.len() - 1] {
        let key = YamlValue::String((*seg).to_string());
        match current.get_mut(&key) {
            Some(YamlValue::Mapping(next_map)) => current = next_map,
            None => return Ok(false),
            Some(_) => {
                return Err(FrontmatterError::NotAMapping {
                    key: (*seg).to_string(),
                    path: path.to_string(),
                });
            }
        }
    }

    let last_key = YamlValue::String(segments[segments.len() - 1].to_string());
    match current.get_mut(&last_key) {
        None => Ok(false),
        Some(YamlValue::Sequence(seq)) => {
            if seq.len() <= 1 {
                return Ok(false);
            }
            let original_json: Vec<JsonValue> = seq
                .iter()
                .map(yaml_to_json)
                .collect::<Result<Vec<_>, _>>()?;
            let mut paired: Vec<(JsonValue, YamlValue)> =
                original_json.iter().cloned().zip(seq.drain(..)).collect();
            paired.sort_by(|(a, _), (b, _)| compare_json_values(a, b));

            let mut deduped: Vec<YamlValue> = Vec::with_capacity(paired.len());
            let mut deduped_json: Vec<JsonValue> = Vec::with_capacity(paired.len());
            for (json_v, yaml_v) in paired {
                if deduped_json.last() != Some(&json_v) {
                    deduped_json.push(json_v);
                    deduped.push(yaml_v);
                }
            }
            let changed = deduped_json != original_json;
            *seq = deduped;
            Ok(changed)
        }
        Some(_) => Err(FrontmatterError::NotAnArray {
            path: path.to_string(),
        }),
    }
}

/// Convert a `toml_edit::DocumentMut` into a YAML `Mapping`, preserving key order.
pub fn from_toml(doc: &toml_edit::DocumentMut) -> Mapping {
    toml_table_to_yaml_mapping(doc.as_table())
}

fn toml_table_to_yaml_mapping(table: &dyn toml_edit::TableLike) -> Mapping {
    let mut map = Mapping::new();
    for (k, item) in table.iter() {
        if let Some(val) = toml_item_to_yaml(item) {
            map.insert(YamlValue::String(k.to_string()), val);
        }
    }
    map
}

fn toml_item_to_yaml(item: &toml_edit::Item) -> Option<YamlValue> {
    match item {
        toml_edit::Item::None => None,
        toml_edit::Item::Value(v) => Some(toml_value_to_yaml(v)),
        toml_edit::Item::Table(t) => Some(YamlValue::Mapping(toml_table_to_yaml_mapping(t))),
        toml_edit::Item::ArrayOfTables(aot) => {
            let seq = aot
                .iter()
                .map(|t| YamlValue::Mapping(toml_table_to_yaml_mapping(t)))
                .collect();
            Some(YamlValue::Sequence(seq))
        }
    }
}

fn toml_value_to_yaml(val: &toml_edit::Value) -> YamlValue {
    match val {
        toml_edit::Value::String(s) => YamlValue::String(s.value().clone()),
        toml_edit::Value::Integer(i) => YamlValue::Number((*i.value()).into()),
        toml_edit::Value::Float(f) => YamlValue::Number((*f.value()).into()),
        toml_edit::Value::Boolean(b) => YamlValue::Bool(*b.value()),
        toml_edit::Value::Datetime(dt) => YamlValue::String(dt.value().to_string()),
        toml_edit::Value::Array(arr) => {
            YamlValue::Sequence(arr.iter().map(toml_value_to_yaml).collect())
        }
        toml_edit::Value::InlineTable(inline) => {
            let mut map = Mapping::with_capacity(inline.len());
            for (k, v) in inline.iter() {
                map.insert(YamlValue::String(k.to_string()), toml_value_to_yaml(v));
            }
            YamlValue::Mapping(map)
        }
    }
}

/// Query a dot-separated key path from a YAML mapping and convert the result to JSON.
pub fn get(mapping: &Mapping, path: &str) -> Option<JsonValue> {
    let segments = split_key_path(path).ok()?;
    let mut current = mapping.get(YamlValue::String(segments[0].to_string()))?;

    for seg in &segments[1..] {
        match current {
            YamlValue::Mapping(map) => {
                current = map.get(YamlValue::String((*seg).to_string()))?;
            }
            YamlValue::Sequence(seq) => {
                let idx = seg.parse::<usize>().ok()?;
                current = seq.get(idx)?;
            }
            _ => return None,
        }
    }

    yaml_to_json(current).ok()
}

fn ensure_parent_mapping<'a>(
    mapping: &'a mut Mapping,
    parent_segments: &[&str],
    full_path: &str,
) -> Result<&'a mut Mapping, FrontmatterError> {
    let mut current = mapping;
    for seg in parent_segments {
        let key = YamlValue::String((*seg).to_string());
        if !current.contains_key(&key) {
            current.insert(key.clone(), YamlValue::Mapping(Mapping::new()));
        }

        let entry = current.get_mut(&key).expect("key was just ensured");
        match entry {
            YamlValue::Mapping(next_map) => current = next_map,
            _ => {
                return Err(FrontmatterError::NotAMapping {
                    key: (*seg).to_string(),
                    path: full_path.to_string(),
                });
            }
        }
    }
    Ok(current)
}

/// Insert or update a dot-separated key path in a YAML mapping.
pub fn set(mapping: &mut Mapping, path: &str, value: JsonValue) -> Result<(), FrontmatterError> {
    let segments = split_key_path(path)?;
    let current = ensure_parent_mapping(mapping, &segments[..segments.len() - 1], path)?;

    let last = segments[segments.len() - 1];
    current.insert(YamlValue::String(last.to_string()), json_to_yaml(&value));
    Ok(())
}

/// Append item(s) to a YAML sequence at `path`, deduplicating existing entries.
///
/// Creates intermediate mappings and a new sequence if the target key is absent.
pub fn append(
    mapping: &mut Mapping,
    path: &str,
    items: &[JsonValue],
) -> Result<(), FrontmatterError> {
    let segments = split_key_path(path)?;
    let current = ensure_parent_mapping(mapping, &segments[..segments.len() - 1], path)?;

    let last_key = YamlValue::String(segments[segments.len() - 1].to_string());
    match current.get_mut(&last_key) {
        Some(YamlValue::Sequence(seq)) => {
            let mut existing: Vec<JsonValue> = seq
                .iter()
                .map(yaml_to_json)
                .collect::<Result<Vec<_>, _>>()?;
            for item in items {
                if !existing.iter().any(|e| e == item) {
                    seq.push(json_to_yaml(item));
                    existing.push(item.clone());
                }
            }
            Ok(())
        }
        Some(_) => Err(FrontmatterError::NotAnArray {
            path: path.to_string(),
        }),
        None => {
            let mut seq = Vec::with_capacity(items.len());
            let mut seen: Vec<&JsonValue> = Vec::with_capacity(items.len());
            for item in items {
                if !seen.contains(&item) {
                    seq.push(json_to_yaml(item));
                    seen.push(item);
                }
            }
            current.insert(last_key, YamlValue::Sequence(seq));
            Ok(())
        }
    }
}

/// Remove matching item(s) from a YAML sequence at `path`.
///
/// Missing intermediate mappings or target keys are treated as no-ops.
pub fn remove_from_array(
    mapping: &mut Mapping,
    path: &str,
    items: &[JsonValue],
) -> Result<(), FrontmatterError> {
    let segments = split_key_path(path)?;
    let mut current = mapping;

    for seg in &segments[..segments.len() - 1] {
        let key = YamlValue::String((*seg).to_string());
        match current.get_mut(&key) {
            Some(YamlValue::Mapping(next_map)) => current = next_map,
            None => return Ok(()),
            Some(_) => {
                return Err(FrontmatterError::NotAMapping {
                    key: (*seg).to_string(),
                    path: path.to_string(),
                });
            }
        }
    }

    let last_key = YamlValue::String(segments[segments.len() - 1].to_string());
    match current.get_mut(&last_key) {
        None => Ok(()),
        Some(YamlValue::Sequence(seq)) => {
            let mut kept = Vec::with_capacity(seq.len());
            for elem in seq.drain(..) {
                let elem_json = yaml_to_json(&elem)?;
                if !items.iter().any(|target| target == &elem_json) {
                    kept.push(elem);
                }
            }
            *seq = kept;
            Ok(())
        }
        Some(_) => Err(FrontmatterError::NotAnArray {
            path: path.to_string(),
        }),
    }
}

/// Delete a dot-separated key path from a YAML mapping.
///
/// Returns `true` if the key existed and was removed.
pub fn del(mapping: &mut Mapping, path: &str) -> Result<bool, FrontmatterError> {
    let segments = split_key_path(path)?;
    let mut current = mapping;

    for seg in &segments[..segments.len() - 1] {
        let key = YamlValue::String((*seg).to_string());
        match current.get_mut(&key) {
            Some(YamlValue::Mapping(next_map)) => current = next_map,
            _ => return Ok(false),
        }
    }

    let last = segments[segments.len() - 1];
    Ok(current
        .shift_remove(YamlValue::String(last.to_string()))
        .is_some())
}

/// Convert a YAML `Mapping` into a `serde_json::Value::Object`.
pub fn mapping_to_json(mapping: &Mapping) -> Result<JsonValue, FrontmatterError> {
    let mut obj = JsonMap::with_capacity(mapping.len());
    for (k, v) in mapping {
        let key_str = yaml_key_to_string(k);
        obj.insert(key_str, yaml_to_json(v)?);
    }
    Ok(JsonValue::Object(obj))
}

pub(crate) fn yaml_key_to_string(key: &YamlValue) -> String {
    match key {
        YamlValue::String(s) => s.clone(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::Null => "null".to_string(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

pub(crate) fn yaml_to_json(value: &YamlValue) -> Result<JsonValue, FrontmatterError> {
    Ok(match value {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(*b),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                JsonValue::Number(JsonNumber::from(i))
            } else if let Some(u) = n.as_u64() {
                JsonValue::Number(JsonNumber::from(u))
            } else if let Some(f) = n.as_f64() {
                JsonNumber::from_f64(f)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            } else {
                JsonValue::Null
            }
        }
        YamlValue::String(s) => JsonValue::String(s.clone()),
        YamlValue::Sequence(seq) => {
            let mut arr = Vec::with_capacity(seq.len());
            for item in seq {
                arr.push(yaml_to_json(item)?);
            }
            JsonValue::Array(arr)
        }
        YamlValue::Mapping(map) => mapping_to_json(map)?,
        YamlValue::Tagged(tagged) => yaml_to_json(&tagged.value)?,
    })
}

pub(crate) fn json_to_yaml(value: &JsonValue) -> YamlValue {
    match value {
        JsonValue::Null => YamlValue::Null,
        JsonValue::Bool(b) => YamlValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                YamlValue::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                YamlValue::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                YamlValue::Number(f.into())
            } else {
                YamlValue::Null
            }
        }
        JsonValue::String(s) => YamlValue::String(s.clone()),
        JsonValue::Array(arr) => YamlValue::Sequence(arr.iter().map(json_to_yaml).collect()),
        JsonValue::Object(map) => {
            let mut yaml_map = Mapping::with_capacity(map.len());
            for (k, v) in map {
                yaml_map.insert(YamlValue::String(k.clone()), json_to_yaml(v));
            }
            YamlValue::Mapping(yaml_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn yaml_dot_path_get_set_del() {
        let mut map = parse_yaml("title: Test\nauthor:\n  name: Alice\ntags:\n  - rust\n").unwrap();
        assert_eq!(get(&map, "title"), Some(json!("Test")));
        assert_eq!(get(&map, "author.name"), Some(json!("Alice")));
        assert_eq!(get(&map, "tags.0"), Some(json!("rust")));
        assert_eq!(get(&map, "missing.key"), None);

        set(&mut map, "author.email", json!("alice@example.com")).unwrap();
        set(&mut map, "meta.nested.count", json!(3)).unwrap();
        assert_eq!(get(&map, "author.email"), Some(json!("alice@example.com")));
        assert_eq!(get(&map, "meta.nested.count"), Some(json!(3)));

        assert!(del(&mut map, "author.email").unwrap());
        assert!(!del(&mut map, "author.email").unwrap());
        assert_eq!(get(&map, "author.email"), None);

        // Traversing through a scalar returns NotAMapping.
        assert!(matches!(
            set(&mut map, "title.sub", json!(1)),
            Err(FrontmatterError::NotAMapping { .. })
        ));
    }

    #[test]
    fn yaml_edge_cases_types_keys_and_errors() {
        // Empty and null YAML parse to empty mapping; sequence root errors.
        assert!(parse_yaml("  \n").unwrap().is_empty());
        assert!(parse_yaml("~\n").unwrap().is_empty());
        assert_eq!(serialize_yaml(&Mapping::new()).unwrap(), "");
        assert!(matches!(
            parse_yaml("- a\n- b\n"),
            Err(FrontmatterError::InvalidRoot {
                format: Format::Yaml
            })
        ));

        // Non-string keys, floats, unsigned ints, null, tagged values.
        let raw = "\
true: bool_key
42: num_key
null: null_key
[1, 2]: seq_key
float_val: 1.5
nan_val: .nan
big_u64: 18446744073709551615
empty: null
tagged: !custom hello
scalar: text
";
        let mut map = parse_yaml(raw).unwrap();
        let json_obj = mapping_to_json(&map).unwrap();
        assert_eq!(json_obj["true"], json!("bool_key"));
        assert_eq!(json_obj["42"], json!("num_key"));
        assert_eq!(json_obj["null"], json!("null_key"));
        assert_eq!(json_obj["float_val"], json!(1.5));
        assert_eq!(json_obj["nan_val"], json!(null));
        assert_eq!(json_obj["big_u64"], json!(u64::MAX));
        assert_eq!(json_obj["empty"], json!(null));
        assert_eq!(json_obj["tagged"], json!("hello"));

        // Traversing through scalar in get/del.
        assert_eq!(get(&map, "scalar.sub"), None);
        assert!(!del(&mut map, "missing.sub").unwrap());
        assert!(!del(&mut map, "scalar.sub").unwrap());

        // json_to_yaml for Null, u64, f64, Array, Object.
        set(
            &mut map,
            "complex",
            json!({
                "n": null,
                "u": u64::MAX,
                "f": 2.75,
                "arr": [1, true, "s"]
            }),
        )
        .unwrap();
        assert_eq!(get(&map, "complex.n"), Some(json!(null)));
        assert_eq!(get(&map, "complex.u"), Some(json!(u64::MAX)));
        assert_eq!(get(&map, "complex.f"), Some(json!(2.75)));
        assert_eq!(get(&map, "complex.arr.2"), Some(json!("s")));
    }

    #[test]
    fn yaml_del_preserves_key_order() {
        let raw = "\
source: Recipe Box
servings: 1
prep time: 5 minutes
cook time: 0 minutes
total time: 5 minutes
meta:
  first: 1
  second: 2
  third: 3
hero: assets/images/fireside-special.webp
";
        let mut map = parse_yaml(raw).unwrap();
        assert!(del(&mut map, "prep time").unwrap());
        assert!(del(&mut map, "meta.second").unwrap());

        let serialized = serialize_yaml(&map).unwrap();
        let keys: Vec<&str> = map
            .keys()
            .map(|k| k.as_str().expect("string key"))
            .collect();
        assert_eq!(
            keys,
            vec![
                "source",
                "servings",
                "cook time",
                "total time",
                "meta",
                "hero"
            ]
        );

        let expected = "\
source: Recipe Box
servings: 1
cook time: 0 minutes
total time: 5 minutes
meta:
  first: 1
  third: 3
hero: assets/images/fireside-special.webp
";
        assert_eq!(serialized, expected);
    }

    #[test]
    fn yaml_array_append_and_remove() {
        let mut map = parse_yaml("title: Bread\ntags:\n  - bread\n  - draft\n").unwrap();

        // Append single + deduplicate existing and repeated items.
        append(&mut map, "tags", &[json!("must-try")]).unwrap();
        append(
            &mut map,
            "tags",
            &[json!("bread"), json!("featured"), json!("featured")],
        )
        .unwrap();
        assert_eq!(
            get(&map, "tags"),
            Some(json!(["bread", "draft", "must-try", "featured"]))
        );

        // Append creates missing top-level and nested arrays (deduplicating input).
        append(&mut map, "categories", &[json!("baking"), json!("baking")]).unwrap();
        append(&mut map, "meta.labels", &[json!("v1")]).unwrap();
        assert_eq!(get(&map, "categories"), Some(json!(["baking"])));
        assert_eq!(get(&map, "meta.labels"), Some(json!(["v1"])));

        // Remove single and multiple items while preserving remaining order.
        remove_from_array(&mut map, "tags", &[json!("draft"), json!("featured")]).unwrap();
        assert_eq!(get(&map, "tags"), Some(json!(["bread", "must-try"])));

        // Remove missing item, empty array, missing key, and missing intermediate path are no-ops.
        remove_from_array(&mut map, "tags", &[json!("nonexistent")]).unwrap();
        remove_from_array(&mut map, "missing_key", &[json!("a")]).unwrap();
        remove_from_array(&mut map, "absent.nested", &[json!("a")]).unwrap();
        assert_eq!(get(&map, "absent"), None);

        remove_from_array(&mut map, "categories", &[json!("baking")]).unwrap();
        assert_eq!(get(&map, "categories"), Some(json!([])));
        remove_from_array(&mut map, "categories", &[json!("baking")]).unwrap();
        assert_eq!(get(&map, "categories"), Some(json!([])));

        // Type errors on non-array target and non-mapping intermediate segments.
        assert!(matches!(
            append(&mut map, "title", &[json!("x")]),
            Err(FrontmatterError::NotAnArray { .. })
        ));
        assert!(matches!(
            remove_from_array(&mut map, "title", &[json!("x")]),
            Err(FrontmatterError::NotAnArray { .. })
        ));
        assert!(matches!(
            append(&mut map, "title.tags", &[json!("x")]),
            Err(FrontmatterError::NotAMapping { .. })
        ));
        assert!(matches!(
            remove_from_array(&mut map, "title.tags", &[json!("x")]),
            Err(FrontmatterError::NotAMapping { .. })
        ));
    }

    #[test]
    fn yaml_indent_options_sequences_nested_mappings_and_block_scalars() {
        let mut map = Mapping::new();
        set(&mut map, "title", json!(":cake: Pumpkin Cake")).unwrap();
        set(&mut map, "tags", json!(["rust", "cli"])).unwrap();
        set(&mut map, "empty_seq", json!([])).unwrap();
        set(&mut map, "empty_map", json!({})).unwrap();
        set(
            &mut map,
            "authors",
            json!([
                {"name": "Alice", "tags": ["a", "b"]},
                {"name": "Bob", "role": "ops"}
            ]),
        )
        .unwrap();
        set(&mut map, "matrix", json!([[1, 2], [3, 4]])).unwrap();
        set(&mut map, "notes", json!("Line 1\n- dash in scalar\nLine 3")).unwrap();
        set(&mut map, "indented_note", json!("  leading\nline 2")).unwrap();
        set(
            &mut map,
            "seq_notes",
            json!(["first line\nsecond line", "  indented\nline 2"]),
        )
        .unwrap();

        let original_json = mapping_to_json(&map).unwrap();

        for opts in [
            YamlFormatOptions {
                indent_sequences: true,
                yaml_indent: 2,
            },
            YamlFormatOptions {
                indent_sequences: false,
                yaml_indent: 2,
            },
            YamlFormatOptions {
                indent_sequences: true,
                yaml_indent: 4,
            },
        ] {
            let serialized = serialize_yaml_with_options(&map, opts).unwrap();
            let reparsed = parse_yaml(&serialized).unwrap();
            assert_eq!(
                mapping_to_json(&reparsed).unwrap(),
                original_json,
                "round-trip failed for opts={opts:?}:\n{serialized}"
            );
            // Idempotency check.
            let second = serialize_yaml_with_options(&reparsed, opts).unwrap();
            assert_eq!(
                second, serialized,
                "not idempotent for opts={opts:?}:\nfirst:\n{serialized}\nsecond:\n{second}"
            );
        }

        // Verify exact 2-space indented sequence layout.
        let s2 = serialize_yaml_with_options(&map, YamlFormatOptions::default()).unwrap();
        assert!(s2.contains("tags:\n  - rust\n  - cli\n"));
        assert!(s2.contains("authors:\n  - name: Alice\n    tags:\n      - a\n      - b\n"));

        // Verify unindented sequence layout when indent_sequences = false.
        let s_unindented = serialize_yaml_with_options(
            &map,
            YamlFormatOptions {
                indent_sequences: false,
                yaml_indent: 2,
            },
        )
        .unwrap();
        assert!(s_unindented.contains("tags:\n- rust\n- cli\n"));
        assert!(s_unindented.contains("authors:\n- name: Alice\n  tags:\n  - a\n  - b\n"));

        // Verify sort_keys and sort_array.
        let mut s_map = parse_yaml("z: 1\na: 2\ntags:\n  - b\n  - a\n  - b\n").unwrap();
        assert!(sort_array(&mut s_map, "tags").unwrap());
        assert!(!sort_array(&mut s_map, "tags").unwrap());
        assert!(!sort_array(&mut s_map, "missing").unwrap());
        assert!(matches!(
            sort_array(&mut s_map, "z"),
            Err(FrontmatterError::NotAnArray { .. })
        ));
        sort_keys(&mut s_map);
        let keys: Vec<String> = s_map.keys().map(yaml_key_to_string).collect();
        assert_eq!(keys, vec!["a", "tags", "z"]);
        assert_eq!(get(&s_map, "tags"), Some(json!(["a", "b"])));
    }
}
