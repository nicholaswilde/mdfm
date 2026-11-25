use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cli::{AuditSort, OutputFormat};
use crate::commands::{build_table, collect_markdown_files, format_display_value};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::{FilterContext, FilterExpr};
use crate::frontmatter::{self, FrontmatterData};

#[derive(Debug, Serialize)]
struct KeyEntry {
    key: String,
    count: usize,
    types: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ValueEntry {
    value: JsonValue,
    count: usize,
    #[serde(skip)]
    display: String,
    #[serde(skip)]
    canonical: String,
}

fn json_type_name(val: &JsonValue) -> &'static str {
    match val {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

fn collect_object_keys(
    obj: &JsonMap<String, JsonValue>,
    prefix: &str,
    per_file: &mut BTreeMap<String, BTreeSet<&'static str>>,
) {
    for (k, v) in obj {
        let full_key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        per_file
            .entry(full_key.clone())
            .or_default()
            .insert(json_type_name(v));
        if let JsonValue::Object(child) = v {
            collect_object_keys(child, &full_key, per_file);
        }
    }
}

/// Audit frontmatter keys, file counts, and observed JSON types across files.
pub fn execute_keys(
    paths: &[PathBuf],
    sort: AuditSort,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let files = collect_markdown_files(paths, &config.ignore, &config.extensions)?;
    let mut stats: BTreeMap<String, (usize, BTreeSet<&'static str>)> = BTreeMap::new();
    let mut matched_files: Vec<String> = Vec::new();

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&content);
        let fm = doc
            .parse_frontmatter()
            .with_context(|| format!("Failed to parse frontmatter in '{}'", path.display()))?;

        let fm_json = fm
            .as_ref()
            .map(FrontmatterData::to_json_value)
            .transpose()?;

        if let Some(expr) = filter {
            let ctx = FilterContext {
                frontmatter: fm_json.as_ref(),
                format: fm.as_ref().map(FrontmatterData::format),
            };
            if !expr.matches_doc(ctx) {
                continue;
            }
        }

        let Some(JsonValue::Object(obj)) = fm_json else {
            continue;
        };

        let mut per_file: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
        collect_object_keys(&obj, "", &mut per_file);

        if !per_file.is_empty() {
            matched_files.push(path.display().to_string());
        }

        for (key, types) in per_file {
            let entry = stats.entry(key).or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.extend(types);
        }
    }

    let mut entries: Vec<KeyEntry> = stats
        .into_iter()
        .map(|(key, (count, types))| KeyEntry {
            key,
            count,
            types: types.into_iter().collect(),
        })
        .collect();

    match sort {
        AuditSort::Count => {
            entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
        }
        AuditSort::Alpha => {
            entries.sort_by(|a, b| a.key.cmp(&b.key));
        }
    }

    match config.output {
        OutputFormat::Files => {
            for file in matched_files {
                writeln!(out, "{file}")?;
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&entries)?)?;
        }
        OutputFormat::Plain => {
            for entry in &entries {
                writeln!(
                    out,
                    "{}\t{}\t{}",
                    entry.key,
                    entry.count,
                    entry.types.join(", ")
                )?;
            }
        }
        OutputFormat::Table => {
            if entries.is_empty() {
                return Ok(());
            }
            let mut table = build_table(&["Key", "Count", "Types"]);
            for entry in &entries {
                table.add_row(vec![
                    entry.key.clone(),
                    entry.count.to_string(),
                    entry.types.join(", "),
                ]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}

/// Audit unique values and occurrence counts for a specific frontmatter key across files.
pub fn execute_values(
    key: &str,
    paths: &[PathBuf],
    no_flatten: bool,
    sort: AuditSort,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    if key.is_empty() || key.split('.').any(str::is_empty) {
        anyhow::bail!("Invalid key path: '{key}'");
    }

    let files = collect_markdown_files(paths, &config.ignore, &config.extensions)?;
    let mut counts: BTreeMap<String, (JsonValue, usize)> = BTreeMap::new();
    let mut matched_files: Vec<String> = Vec::new();

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&content);
        let fm = doc
            .parse_frontmatter()
            .with_context(|| format!("Failed to parse frontmatter in '{}'", path.display()))?;

        let fm_json = fm
            .as_ref()
            .map(FrontmatterData::to_json_value)
            .transpose()?;

        if let Some(expr) = filter {
            let ctx = FilterContext {
                frontmatter: fm_json.as_ref(),
                format: fm.as_ref().map(FrontmatterData::format),
            };
            if !expr.matches_doc(ctx) {
                continue;
            }
        }

        let Some(fm) = fm else {
            continue;
        };

        let Some(val) = fm.get(key) else {
            continue;
        };

        matched_files.push(path.display().to_string());

        let record = |item: JsonValue, map: &mut BTreeMap<String, (JsonValue, usize)>| {
            let canonical = serde_json::to_string(&item).unwrap_or_default();
            let entry = map.entry(canonical).or_insert_with(|| (item, 0));
            entry.1 += 1;
        };

        match val {
            JsonValue::Array(items) if !no_flatten => {
                for item in items {
                    record(item, &mut counts);
                }
            }
            other => record(other, &mut counts),
        }
    }

    let mut entries: Vec<ValueEntry> = counts
        .into_iter()
        .map(|(canonical, (value, count))| {
            let display = format_display_value(&value);
            ValueEntry {
                value,
                count,
                display,
                canonical,
            }
        })
        .collect();

    match sort {
        AuditSort::Count => {
            entries.sort_by(|a, b| {
                b.count
                    .cmp(&a.count)
                    .then_with(|| a.display.cmp(&b.display))
                    .then_with(|| a.canonical.cmp(&b.canonical))
            });
        }
        AuditSort::Alpha => {
            entries.sort_by(|a, b| {
                a.display
                    .cmp(&b.display)
                    .then_with(|| a.canonical.cmp(&b.canonical))
            });
        }
    }

    match config.output {
        OutputFormat::Files => {
            for file in matched_files {
                writeln!(out, "{file}")?;
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&entries)?)?;
        }
        OutputFormat::Plain => {
            for entry in &entries {
                writeln!(out, "{}\t{}", entry.display, entry.count)?;
            }
        }
        OutputFormat::Table => {
            if entries.is_empty() {
                return Ok(());
            }
            let mut table = build_table(&["Value", "Count"]);
            for entry in &entries {
                table.add_row(vec![entry.display.clone(), entry.count.to_string()]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_counts_nested_mixed_types_sorting_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        let c = dir.path().join("c.md");
        let plain = dir.path().join("plain.md");

        fs::write(
            &a,
            "---\ntitle: One\ndraft: false\ntags:\n  - rust\n  - cli\nauthor:\n  name: Alice\n  role: dev\n---\nBody\n",
        )
        .unwrap();
        fs::write(
            &b,
            "+++\ntitle = \"Two\"\ndraft = true\ntags = \"rust,cli\"\ntag = \"typo\"\n[author]\nname = \"Bob\"\n+++\nBody\n",
        )
        .unwrap();
        fs::write(&c, "---\ntitle: Three\nweight: 10\nnote: null\n---\nBody\n").unwrap();
        fs::write(&plain, "# Body only\n").unwrap();

        // JSON output sorted by count descending.
        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute_keys(
            &[dir.path().to_path_buf()],
            AuditSort::Count,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["key"], json!("title"));
        assert_eq!(parsed[0]["count"], json!(3));
        assert_eq!(parsed[0]["types"], json!(["string"]));

        let tags = parsed.iter().find(|e| e["key"] == "tags").unwrap();
        assert_eq!(tags["count"], json!(2));
        assert_eq!(tags["types"], json!(["array", "string"]));

        let author_name = parsed.iter().find(|e| e["key"] == "author.name").unwrap();
        assert_eq!(author_name["count"], json!(2));
        assert_eq!(author_name["types"], json!(["string"]));

        let tag = parsed.iter().find(|e| e["key"] == "tag").unwrap();
        assert_eq!(tag["count"], json!(1));

        let weight = parsed.iter().find(|e| e["key"] == "weight").unwrap();
        assert_eq!(weight["types"], json!(["number"]));

        let note = parsed.iter().find(|e| e["key"] == "note").unwrap();
        assert_eq!(note["types"], json!(["null"]));

        // Alpha sort places `author` first.
        let mut alpha_out = Vec::new();
        execute_keys(
            &[dir.path().to_path_buf()],
            AuditSort::Alpha,
            None,
            &json_cfg,
            &mut alpha_out,
        )
        .unwrap();
        let alpha_parsed: Vec<JsonValue> = serde_json::from_slice(&alpha_out).unwrap();
        assert_eq!(alpha_parsed[0]["key"], json!("author"));
        assert_eq!(alpha_parsed[1]["key"], json!("author.name"));

        // --where filter restricts files audited.
        let filter = FilterExpr::parse("draft == true").unwrap();
        let mut filtered_out = Vec::new();
        execute_keys(
            &[dir.path().to_path_buf()],
            AuditSort::Alpha,
            Some(&filter),
            &json_cfg,
            &mut filtered_out,
        )
        .unwrap();
        let filtered: Vec<JsonValue> = serde_json::from_slice(&filtered_out).unwrap();
        assert!(filtered.iter().any(|e| e["key"] == "tag"));
        assert!(!filtered.iter().any(|e| e["key"] == "weight"));

        // Plain, table, and files outputs.
        let plain_cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            ..ResolvedConfig::default()
        };
        let mut plain_out = Vec::new();
        execute_keys(
            &[dir.path().to_path_buf()],
            AuditSort::Count,
            None,
            &plain_cfg,
            &mut plain_out,
        )
        .unwrap();
        let plain_str = String::from_utf8(plain_out).unwrap();
        assert!(plain_str.contains("title\t3\tstring\n"));
        assert!(plain_str.contains("tags\t2\tarray, string\n"));

        let table_cfg = ResolvedConfig {
            output: OutputFormat::Table,
            ..ResolvedConfig::default()
        };
        let mut table_out = Vec::new();
        execute_keys(
            &[dir.path().to_path_buf()],
            AuditSort::Count,
            None,
            &table_cfg,
            &mut table_out,
        )
        .unwrap();
        let table_str = String::from_utf8(table_out).unwrap();
        assert!(table_str.contains("Key"));
        assert!(table_str.contains("array, string"));

        let files_cfg = ResolvedConfig {
            output: OutputFormat::Files,
            ..ResolvedConfig::default()
        };
        let mut files_out = Vec::new();
        execute_keys(
            &[dir.path().to_path_buf()],
            AuditSort::Count,
            None,
            &files_cfg,
            &mut files_out,
        )
        .unwrap();
        let files_lines: Vec<&str> = std::str::from_utf8(&files_out).unwrap().lines().collect();
        assert_eq!(files_lines.len(), 3);
    }

    #[test]
    fn values_flattens_arrays_supports_no_flatten_nested_sort_and_where() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.md"),
            "---\ndraft: false\ntags:\n  - side\n  - rust\nauthor:\n  role: dev\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("b.md"),
            "---\ndraft: false\ntags:\n  - rust\n  - sides\nauthor:\n  role: ops\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("c.md"),
            "---\ndraft: true\ntags:\n  - rust\n  - side\nauthor:\n  role: dev\n---\n",
        )
        .unwrap();

        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };

        // Default flattens arrays and sorts by count descending.
        let mut out = Vec::new();
        execute_values(
            "tags",
            &[dir.path().to_path_buf()],
            false,
            AuditSort::Count,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            parsed,
            vec![
                json!({"value": "rust", "count": 3}),
                json!({"value": "side", "count": 2}),
                json!({"value": "sides", "count": 1}),
            ]
        );

        // --no-flatten keeps whole arrays intact.
        let mut nf_out = Vec::new();
        execute_values(
            "tags",
            &[dir.path().to_path_buf()],
            true,
            AuditSort::Count,
            None,
            &json_cfg,
            &mut nf_out,
        )
        .unwrap();
        let nf_parsed: Vec<JsonValue> = serde_json::from_slice(&nf_out).unwrap();
        assert_eq!(nf_parsed.len(), 3);
        assert!(nf_parsed.iter().all(|e| e["value"].is_array()));

        // Nested key + --where + alpha sort.
        let filter = FilterExpr::parse("draft == false").unwrap();
        let mut nested_out = Vec::new();
        execute_values(
            "author.role",
            &[dir.path().to_path_buf()],
            false,
            AuditSort::Alpha,
            Some(&filter),
            &json_cfg,
            &mut nested_out,
        )
        .unwrap();
        let nested_parsed: Vec<JsonValue> = serde_json::from_slice(&nested_out).unwrap();
        assert_eq!(
            nested_parsed,
            vec![
                json!({"value": "dev", "count": 1}),
                json!({"value": "ops", "count": 1}),
            ]
        );

        // Plain, table, files outputs and invalid key error.
        let plain_cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            ..ResolvedConfig::default()
        };
        let mut plain_out = Vec::new();
        execute_values(
            "tags",
            &[dir.path().to_path_buf()],
            false,
            AuditSort::Alpha,
            None,
            &plain_cfg,
            &mut plain_out,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(plain_out).unwrap(),
            "rust\t3\nside\t2\nsides\t1\n"
        );

        let table_cfg = ResolvedConfig {
            output: OutputFormat::Table,
            ..ResolvedConfig::default()
        };
        let mut table_out = Vec::new();
        execute_values(
            "tags",
            &[dir.path().to_path_buf()],
            false,
            AuditSort::Count,
            None,
            &table_cfg,
            &mut table_out,
        )
        .unwrap();
        let table_str = String::from_utf8(table_out).unwrap();
        assert!(table_str.contains("Value"));
        assert!(table_str.contains("rust"));

        assert!(execute_values(
            "bad..key",
            &[dir.path().to_path_buf()],
            false,
            AuditSort::Count,
            None,
            &json_cfg,
            &mut Vec::new(),
        )
        .is_err());
    }
}
