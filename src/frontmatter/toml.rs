use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, TableLike, Value as TomlValue};

use super::{split_key_path, Format, FrontmatterError};

/// Parse TOML frontmatter text into a format-preserving `toml_edit::DocumentMut`.
pub fn parse_toml(raw: &str) -> Result<DocumentMut, FrontmatterError> {
    if raw.trim().is_empty() {
        return Ok(DocumentMut::new());
    }
    Ok(raw.parse::<DocumentMut>()?)
}

/// Serialize a `toml_edit::DocumentMut` back to its format-preserving TOML representation.
pub fn serialize_toml(doc: &DocumentMut) -> String {
    doc.to_string()
}

/// Query a dot-separated key path from a TOML document and convert the result to JSON.
pub fn get(doc: &DocumentMut, path: &str) -> Option<JsonValue> {
    let segments = split_key_path(path).ok()?;
    let mut current: &Item = doc.as_table().get(segments[0])?;

    for (idx_seg, seg) in segments.iter().enumerate().skip(1) {
        match current {
            Item::Table(table) => {
                current = table.get(seg)?;
            }
            Item::Value(TomlValue::InlineTable(inline)) => {
                let val = inline.get(seg)?;
                return get_from_toml_value(val, &segments[idx_seg + 1..]);
            }
            Item::ArrayOfTables(aot) => {
                let idx = seg.parse::<usize>().ok()?;
                let tbl = aot.get(idx)?;
                let json_tbl = table_to_json(tbl);
                return lookup_json_segments(&json_tbl, &segments[idx_seg + 1..]).cloned();
            }
            Item::Value(TomlValue::Array(arr)) => {
                let idx = seg.parse::<usize>().ok()?;
                let val = arr.get(idx)?;
                return get_from_toml_value(val, &segments[idx_seg + 1..]);
            }
            _ => return None,
        }
    }

    item_to_json(current)
}

fn get_from_toml_value(val: &TomlValue, remaining: &[&str]) -> Option<JsonValue> {
    let mut cur = val;
    for seg in remaining {
        match cur {
            TomlValue::InlineTable(inline) => {
                cur = inline.get(seg)?;
            }
            TomlValue::Array(arr) => {
                let idx = seg.parse::<usize>().ok()?;
                cur = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(toml_value_to_json(cur))
}

fn lookup_json_segments<'a>(val: &'a JsonValue, segments: &[&str]) -> Option<&'a JsonValue> {
    let mut cur = val;
    for seg in segments {
        match cur {
            JsonValue::Object(map) => cur = map.get(*seg)?,
            JsonValue::Array(arr) => {
                let idx = seg.parse::<usize>().ok()?;
                cur = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(cur)
}

enum TableRef<'a> {
    Table(&'a mut Table),
    Inline(&'a mut InlineTable),
}

fn ensure_parent_table<'a>(
    doc: &'a mut DocumentMut,
    parent_segments: &[&str],
    full_path: &str,
) -> Result<TableRef<'a>, FrontmatterError> {
    let mut current = TableRef::Table(doc.as_table_mut());

    for seg in parent_segments {
        current = match current {
            TableRef::Table(tbl) => {
                if !tbl.contains_key(seg) {
                    tbl.insert(seg, Item::Table(Table::new()));
                }
                match tbl.get_mut(seg).expect("key was just ensured") {
                    Item::Table(next) => TableRef::Table(next),
                    Item::Value(TomlValue::InlineTable(inline)) => TableRef::Inline(inline),
                    _ => {
                        return Err(FrontmatterError::NotAMapping {
                            key: (*seg).to_string(),
                            path: full_path.to_string(),
                        });
                    }
                }
            }
            TableRef::Inline(inline) => {
                if !inline.contains_key(seg) {
                    inline.insert(*seg, TomlValue::InlineTable(InlineTable::new()));
                }
                match inline.get_mut(seg).expect("key was just ensured") {
                    TomlValue::InlineTable(next) => TableRef::Inline(next),
                    _ => {
                        return Err(FrontmatterError::NotAMapping {
                            key: (*seg).to_string(),
                            path: full_path.to_string(),
                        });
                    }
                }
            }
        };
    }

    Ok(current)
}

fn append_to_toml_array(arr: &mut Array, items: &[JsonValue]) -> Result<(), FrontmatterError> {
    let mut existing: Vec<JsonValue> = arr.iter().map(toml_value_to_json).collect();
    for item in items {
        if !existing.iter().any(|e| e == item) {
            let toml_val = json_to_toml_value(item)?;
            arr.push(toml_val);
            existing.push(item.clone());
        }
    }
    Ok(())
}

fn remove_from_toml_array(arr: &mut Array, items: &[JsonValue]) {
    let indices: Vec<usize> = arr
        .iter()
        .enumerate()
        .filter_map(|(idx, val)| {
            let val_json = toml_value_to_json(val);
            items
                .iter()
                .any(|target| target == &val_json)
                .then_some(idx)
        })
        .collect();

    let removed_first = indices.first() == Some(&0);
    for idx in indices.into_iter().rev() {
        arr.remove(idx);
    }

    if removed_first {
        if let Some(first) = arr.get_mut(0) {
            if first.decor().prefix().and_then(|p| p.as_str()) == Some(" ") {
                first.decor_mut().set_prefix("");
            }
        }
    }
}

/// Losslessly insert or update a dot-separated key path in a TOML document.
pub fn set(doc: &mut DocumentMut, path: &str, value: JsonValue) -> Result<(), FrontmatterError> {
    let segments = split_key_path(path)?;
    let current = ensure_parent_table(doc, &segments[..segments.len() - 1], path)?;

    let last = segments[segments.len() - 1];
    let mut new_val = json_to_toml_value(&value)?;

    match current {
        TableRef::Table(tbl) => {
            if let Some(existing_item) = tbl.get_mut(last) {
                if let Item::Value(existing_val) = existing_item {
                    // Preserve key leading comments as well as value whitespace and inline comments.
                    *new_val.decor_mut() = existing_val.decor().clone();
                    *existing_val = new_val;
                } else {
                    *existing_item = toml_edit::value(new_val);
                }
            } else {
                tbl.insert(last, toml_edit::value(new_val));
            }
        }
        TableRef::Inline(inline) => {
            if let Some(existing_val) = inline.get_mut(last) {
                *new_val.decor_mut() = existing_val.decor().clone();
                *existing_val = new_val;
            } else {
                inline.insert(last, new_val);
            }
        }
    }

    Ok(())
}

/// Losslessly append item(s) to a TOML array at `path`, deduplicating existing entries.
///
/// Creates intermediate tables and a new array if the target key is absent.
pub fn append(
    doc: &mut DocumentMut,
    path: &str,
    items: &[JsonValue],
) -> Result<(), FrontmatterError> {
    let segments = split_key_path(path)?;
    let current = ensure_parent_table(doc, &segments[..segments.len() - 1], path)?;
    let last = segments[segments.len() - 1];

    match current {
        TableRef::Table(tbl) => match tbl.get_mut(last) {
            Some(Item::Value(TomlValue::Array(arr))) => append_to_toml_array(arr, items),
            Some(_) => Err(FrontmatterError::NotAnArray {
                path: path.to_string(),
            }),
            None => {
                let mut arr = Array::new();
                append_to_toml_array(&mut arr, items)?;
                tbl.insert(last, toml_edit::value(TomlValue::Array(arr)));
                Ok(())
            }
        },
        TableRef::Inline(inline) => match inline.get_mut(last) {
            Some(TomlValue::Array(arr)) => append_to_toml_array(arr, items),
            Some(_) => Err(FrontmatterError::NotAnArray {
                path: path.to_string(),
            }),
            None => {
                let mut arr = Array::new();
                append_to_toml_array(&mut arr, items)?;
                inline.insert(last, TomlValue::Array(arr));
                Ok(())
            }
        },
    }
}

/// Losslessly remove matching item(s) from a TOML array at `path`.
///
/// Missing intermediate tables or target keys are treated as no-ops.
pub fn remove_from_array(
    doc: &mut DocumentMut,
    path: &str,
    items: &[JsonValue],
) -> Result<(), FrontmatterError> {
    let segments = split_key_path(path)?;
    let mut current = TableRef::Table(doc.as_table_mut());

    for seg in &segments[..segments.len() - 1] {
        current = match current {
            TableRef::Table(tbl) => match tbl.get_mut(seg) {
                None => return Ok(()),
                Some(Item::Table(next)) => TableRef::Table(next),
                Some(Item::Value(TomlValue::InlineTable(inline))) => TableRef::Inline(inline),
                Some(_) => {
                    return Err(FrontmatterError::NotAMapping {
                        key: (*seg).to_string(),
                        path: path.to_string(),
                    });
                }
            },
            TableRef::Inline(inline) => match inline.get_mut(seg) {
                None => return Ok(()),
                Some(TomlValue::InlineTable(next)) => TableRef::Inline(next),
                Some(_) => {
                    return Err(FrontmatterError::NotAMapping {
                        key: (*seg).to_string(),
                        path: path.to_string(),
                    });
                }
            },
        };
    }

    let last = segments[segments.len() - 1];
    match current {
        TableRef::Table(tbl) => match tbl.get_mut(last) {
            None => Ok(()),
            Some(Item::Value(TomlValue::Array(arr))) => {
                remove_from_toml_array(arr, items);
                Ok(())
            }
            Some(_) => Err(FrontmatterError::NotAnArray {
                path: path.to_string(),
            }),
        },
        TableRef::Inline(inline) => match inline.get_mut(last) {
            None => Ok(()),
            Some(TomlValue::Array(arr)) => {
                remove_from_toml_array(arr, items);
                Ok(())
            }
            Some(_) => Err(FrontmatterError::NotAnArray {
                path: path.to_string(),
            }),
        },
    }
}

/// Losslessly delete a dot-separated key path from a TOML document.
///
/// Returns `true` if the key existed and was removed.
pub fn del(doc: &mut DocumentMut, path: &str) -> Result<bool, FrontmatterError> {
    let segments = split_key_path(path)?;
    let mut current = TableRef::Table(doc.as_table_mut());

    for seg in &segments[..segments.len() - 1] {
        current = match current {
            TableRef::Table(tbl) => match tbl.get_mut(seg) {
                Some(Item::Table(next)) => TableRef::Table(next),
                Some(Item::Value(TomlValue::InlineTable(inline))) => TableRef::Inline(inline),
                _ => return Ok(false),
            },
            TableRef::Inline(inline) => match inline.get_mut(seg) {
                Some(TomlValue::InlineTable(next)) => TableRef::Inline(next),
                _ => return Ok(false),
            },
        };
    }

    let last = segments[segments.len() - 1];
    let removed = match current {
        TableRef::Table(tbl) => tbl.remove(last).is_some(),
        TableRef::Inline(inline) => inline.remove(last).is_some(),
    };
    Ok(removed)
}

/// Convert a `toml_edit::DocumentMut` into a `serde_json::Value::Object`.
pub fn document_to_json(doc: &DocumentMut) -> Result<JsonValue, FrontmatterError> {
    Ok(table_to_json(doc.as_table()))
}

fn table_to_json(table: &dyn TableLike) -> JsonValue {
    let mut obj = JsonMap::new();
    for (k, item) in table.iter() {
        if let Some(val) = item_to_json(item) {
            obj.insert(k.to_string(), val);
        }
    }
    JsonValue::Object(obj)
}

fn item_to_json(item: &Item) -> Option<JsonValue> {
    match item {
        Item::None => None,
        Item::Value(v) => Some(toml_value_to_json(v)),
        Item::Table(t) => Some(table_to_json(t)),
        Item::ArrayOfTables(aot) => {
            let arr = aot.iter().map(|t| table_to_json(t)).collect();
            Some(JsonValue::Array(arr))
        }
    }
}

fn toml_value_to_json(val: &TomlValue) -> JsonValue {
    match val {
        TomlValue::String(s) => JsonValue::String(s.value().clone()),
        TomlValue::Integer(i) => JsonValue::Number(JsonNumber::from(*i.value())),
        TomlValue::Float(f) => JsonNumber::from_f64(*f.value())
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        TomlValue::Boolean(b) => JsonValue::Bool(*b.value()),
        TomlValue::Datetime(dt) => JsonValue::String(dt.value().to_string()),
        TomlValue::Array(arr) => JsonValue::Array(arr.iter().map(toml_value_to_json).collect()),
        TomlValue::InlineTable(inline) => {
            let mut obj = JsonMap::with_capacity(inline.len());
            for (k, v) in inline.iter() {
                obj.insert(k.to_string(), toml_value_to_json(v));
            }
            JsonValue::Object(obj)
        }
    }
}

fn json_to_toml_value(val: &JsonValue) -> Result<TomlValue, FrontmatterError> {
    match val {
        JsonValue::Null => Err(FrontmatterError::UnsupportedValue {
            format: Format::Toml,
            reason: "TOML does not support null values".to_string(),
        }),
        JsonValue::Bool(b) => Ok(TomlValue::from(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(TomlValue::from(i))
            } else if let Some(f) = n.as_f64() {
                Ok(TomlValue::from(f))
            } else {
                Err(FrontmatterError::UnsupportedValue {
                    format: Format::Toml,
                    reason: format!("numeric value '{n}' cannot be represented in TOML"),
                })
            }
        }
        JsonValue::String(s) => Ok(TomlValue::from(s.as_str())),
        JsonValue::Array(arr) => {
            let mut toml_arr = Array::new();
            for item in arr {
                toml_arr.push(json_to_toml_value(item)?);
            }
            Ok(TomlValue::Array(toml_arr))
        }
        JsonValue::Object(map) => {
            let mut inline = InlineTable::new();
            for (k, v) in map {
                inline.insert(k, json_to_toml_value(v)?);
            }
            Ok(TomlValue::InlineTable(inline))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn toml_dot_path_tables_and_inline_tables() {
        let raw = "\
# comment
title = \"Hello\"
author = { name = \"Nick\", role = \"dev\" }
tags = [\"rust\", \"cli\"]
";
        let mut doc = parse_toml(raw).unwrap();
        assert_eq!(get(&doc, "title"), Some(json!("Hello")));
        assert_eq!(get(&doc, "author.name"), Some(json!("Nick")));
        assert_eq!(get(&doc, "tags.1"), Some(json!("cli")));

        set(&mut doc, "author.name", json!("Nicholas")).unwrap();
        set(&mut doc, "meta.version", json!(2)).unwrap();
        assert_eq!(get(&doc, "author.name"), Some(json!("Nicholas")));
        assert_eq!(get(&doc, "meta.version"), Some(json!(2)));

        assert!(del(&mut doc, "author.role").unwrap());
        assert!(!del(&mut doc, "author.role").unwrap());
        assert_eq!(get(&doc, "author.role"), None);

        // Null is rejected in TOML.
        assert!(matches!(
            set(&mut doc, "empty", JsonValue::Null),
            Err(FrontmatterError::UnsupportedValue { .. })
        ));
    }

    #[test]
    fn toml_arrays_of_tables_inline_nesting_and_types() {
        assert!(parse_toml("   \n").unwrap().is_empty());

        let raw = "\
title = \"Doc\"
rating = 4.5
published = 2026-11-24T12:00:00Z
nested = { inner = { k = \"v\" }, arr = [10, 20], scalar = 1 }

[section]
key = \"val\"

[[items]]
name = \"first\"
tags = [\"a\", \"b\"]
";
        let mut doc = parse_toml(raw).unwrap();

        // ArrayOfTables, InlineTable nesting, Array indexing, Datetime, Float.
        assert_eq!(get(&doc, "items.0.name"), Some(json!("first")));
        assert_eq!(get(&doc, "items.0.tags.1"), Some(json!("b")));
        assert_eq!(get(&doc, "items.bad"), None);
        assert_eq!(get(&doc, "items.0.name.sub"), None);
        assert_eq!(get(&doc, "nested.inner.k"), Some(json!("v")));
        assert_eq!(get(&doc, "nested.arr.0"), Some(json!(10)));
        assert_eq!(get(&doc, "nested.scalar.sub"), None);
        assert_eq!(get(&doc, "title.sub"), None);
        assert_eq!(get(&doc, "rating"), Some(json!(4.5)));
        assert_eq!(get(&doc, "published"), Some(json!("2026-11-24T12:00:00Z")));

        let full = document_to_json(&doc).unwrap();
        assert_eq!(full["items"][0]["name"], json!("first"));
        assert_eq!(full["nested"]["inner"]["k"], json!("v"));

        // Nested InlineTable set, insert, and NotAMapping errors.
        set(&mut doc, "nested.deep.leaf", json!(true)).unwrap();
        assert_eq!(get(&doc, "nested.deep.leaf"), Some(json!(true)));
        set(&mut doc, "nested.added", json!("ok")).unwrap();
        assert_eq!(get(&doc, "nested.added"), Some(json!("ok")));
        assert!(matches!(
            set(&mut doc, "title.sub", json!(1)),
            Err(FrontmatterError::NotAMapping { .. })
        ));
        assert!(matches!(
            set(&mut doc, "nested.scalar.sub", json!(1)),
            Err(FrontmatterError::NotAMapping { .. })
        ));

        // Delete across standard [section] table before overwriting it.
        assert!(del(&mut doc, "section.key").unwrap());
        assert!(!del(&mut doc, "section.key").unwrap());

        // Overwrite a [table] with a scalar and test float/array/object conversion.
        set(&mut doc, "section", json!("replaced")).unwrap();
        assert_eq!(get(&doc, "section"), Some(json!("replaced")));
        set(&mut doc, "metrics", json!({"pi": 3.25, "nums": [1, 2.5]})).unwrap();
        assert_eq!(get(&doc, "metrics.pi"), Some(json!(3.25)));
        assert_eq!(get(&doc, "metrics.nums.1"), Some(json!(2.5)));

        // Delete across standard table, nested inline table, and missing intermediates.
        assert!(del(&mut doc, "nested.inner.k").unwrap());
        assert!(!del(&mut doc, "nested.inner.k").unwrap());
        assert!(!del(&mut doc, "nested.scalar.k").unwrap());
        assert!(!del(&mut doc, "missing.path").unwrap());
        assert!(del(&mut doc, "rating").unwrap());
        assert!(!del(&mut doc, "rating").unwrap());
    }

    #[test]
    fn toml_array_append_and_remove_lossless() {
        let raw = "\
title = \"TOML Guide\"
tags = [\"rust\", \"draft\"] # keep comment
inline_meta = { labels = [\"a\"] }
";
        let mut doc = parse_toml(raw).unwrap();

        // Append + deduplicate while preserving inline comments.
        append(&mut doc, "tags", &[json!("cli")]).unwrap();
        append(&mut doc, "tags", &[json!("rust"), json!("md"), json!("md")]).unwrap();
        assert_eq!(
            get(&doc, "tags"),
            Some(json!(["rust", "draft", "cli", "md"]))
        );
        let after_append = serialize_toml(&doc);
        assert!(after_append.contains("# keep comment"));

        // Create absent top-level, nested table, and inline table arrays.
        append(&mut doc, "topics", &[json!("docs")]).unwrap();
        append(&mut doc, "meta.tags", &[json!("featured")]).unwrap();
        append(&mut doc, "inline_meta.labels", &[json!("b"), json!("a")]).unwrap();
        append(&mut doc, "inline_meta.extra", &[json!(1)]).unwrap();
        assert_eq!(get(&doc, "topics"), Some(json!(["docs"])));
        assert_eq!(get(&doc, "meta.tags"), Some(json!(["featured"])));
        assert_eq!(get(&doc, "inline_meta.labels"), Some(json!(["a", "b"])));
        assert_eq!(get(&doc, "inline_meta.extra"), Some(json!([1])));

        // Remove first element cleanly without leading stray space + remove inline array item.
        remove_from_array(&mut doc, "tags", &[json!("rust"), json!("draft")]).unwrap();
        assert_eq!(get(&doc, "tags"), Some(json!(["cli", "md"])));
        let serialized = serialize_toml(&doc);
        assert!(
            serialized.contains("tags = [\"cli\", \"md\"] # keep comment"),
            "unexpected TOML output: {serialized}"
        );

        remove_from_array(&mut doc, "inline_meta.labels", &[json!("a")]).unwrap();
        assert_eq!(get(&doc, "inline_meta.labels"), Some(json!(["b"])));

        // Missing keys and missing intermediate tables are no-ops.
        remove_from_array(&mut doc, "missing", &[json!("x")]).unwrap();
        remove_from_array(&mut doc, "no_table.tags", &[json!("x")]).unwrap();
        remove_from_array(&mut doc, "inline_meta.missing", &[json!("x")]).unwrap();
        assert_eq!(get(&doc, "no_table"), None);

        // Non-array, non-mapping, and null-value errors.
        assert!(matches!(
            append(&mut doc, "title", &[json!("x")]),
            Err(FrontmatterError::NotAnArray { .. })
        ));
        assert!(matches!(
            remove_from_array(&mut doc, "title", &[json!("x")]),
            Err(FrontmatterError::NotAnArray { .. })
        ));
        assert!(matches!(
            append(&mut doc, "title.tags", &[json!("x")]),
            Err(FrontmatterError::NotAMapping { .. })
        ));
        assert!(matches!(
            remove_from_array(&mut doc, "title.tags", &[json!("x")]),
            Err(FrontmatterError::NotAMapping { .. })
        ));
        assert!(matches!(
            append(&mut doc, "tags", &[json!(null)]),
            Err(FrontmatterError::UnsupportedValue { .. })
        ));
    }
}
