use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use serde_yaml_ng::{Mapping, Value as YamlValue};

use super::{split_key_path, Format, FrontmatterError};

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

/// Serialize a YAML `Mapping` to a YAML string (without `---` delimiters).
///
/// An empty mapping serializes to `""` so empty frontmatter blocks round-trip cleanly.
pub fn serialize_yaml(mapping: &Mapping) -> Result<String, FrontmatterError> {
    if mapping.is_empty() {
        return Ok(String::new());
    }
    Ok(serde_yaml_ng::to_string(mapping)?)
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

fn yaml_key_to_string(key: &YamlValue) -> String {
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
}
