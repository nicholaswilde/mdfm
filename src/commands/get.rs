use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cli::OutputFormat;
use crate::commands::{build_table, collect_markdown_files, format_display_value};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::{FilterContext, FilterExpr};
use crate::frontmatter::{self, FrontmatterData};

/// Parsed key selector for `mdfm get`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetSelector {
    /// `.` — dump the full frontmatter object.
    Root,
    /// Single dot-path key (backward-compatible output format).
    Single(String),
    /// Multiple dot-path keys queried in a single pass.
    Multi(Vec<String>),
}

#[derive(Debug, Serialize)]
struct GetEntry {
    file: String,
    key: String,
    value: JsonValue,
}

#[derive(Debug, Serialize)]
struct MultiGetEntry {
    file: String,
    values: JsonMap<String, JsonValue>,
    #[serde(skip)]
    ordered: Vec<Option<JsonValue>>,
}

/// Partition `-k`/`--key` flags and positional `args` into a `GetSelector` and target paths.
pub fn parse_get_args(
    flag_keys: &[String],
    args: &[String],
) -> Result<(GetSelector, Vec<PathBuf>)> {
    let (raw_keys, paths): (Vec<&str>, Vec<PathBuf>) = if !flag_keys.is_empty() {
        (
            flag_keys.iter().flat_map(|k| k.split(',')).collect(),
            args.iter().map(PathBuf::from).collect(),
        )
    } else {
        let Some((first, rest)) = args.split_first() else {
            anyhow::bail!("Expected a frontmatter key or '.' (pass <KEY> or -k/--key)");
        };
        (
            first.split(',').collect(),
            rest.iter().map(PathBuf::from).collect(),
        )
    };

    let mut keys = Vec::new();
    for raw in raw_keys {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Invalid empty key selector");
        }
        if !keys.iter().any(|k| k == trimmed) {
            keys.push(trimmed.to_string());
        }
    }

    if keys.is_empty() {
        anyhow::bail!("Expected a frontmatter key or '.' (pass <KEY> or -k/--key)");
    }

    if keys.iter().any(|k| k == ".") {
        if keys.len() > 1 {
            anyhow::bail!("Cannot combine root selector '.' with other keys");
        }
        return Ok((GetSelector::Root, paths));
    }

    if keys.len() == 1 {
        Ok((GetSelector::Single(keys.remove(0)), paths))
    } else {
        Ok((GetSelector::Multi(keys), paths))
    }
}

pub fn execute(
    flag_keys: &[String],
    args: &[String],
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let (selector, paths) = parse_get_args(flag_keys, args)?;
    let single_explicit_file = paths.len() == 1 && paths[0].is_file();
    let files = collect_markdown_files(&paths, &config.ignore, &config.extensions)?;

    match selector {
        GetSelector::Root => execute_single_or_root(
            None,
            &files,
            single_explicit_file,
            filter,
            config.output,
            out,
        ),
        GetSelector::Single(key) => execute_single_or_root(
            Some(&key),
            &files,
            single_explicit_file,
            filter,
            config.output,
            out,
        ),
        GetSelector::Multi(keys) => execute_multi(
            &keys,
            &files,
            single_explicit_file,
            filter,
            config.output,
            out,
        ),
    }
}

fn execute_single_or_root(
    key: Option<&str>,
    files: &[PathBuf],
    single_explicit_file: bool,
    filter: Option<&FilterExpr>,
    output: OutputFormat,
    out: &mut dyn Write,
) -> Result<()> {
    let mut entries = Vec::new();

    for path in files {
        let content = fs::read_to_string(path)
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

        match (key, fm, fm_json) {
            (None, Some(_), Some(val)) => {
                entries.push(GetEntry {
                    file: path.display().to_string(),
                    key: ".".to_string(),
                    value: val,
                });
            }
            (None, None, None) if filter.is_some_and(FilterExpr::references_document_meta) => {
                entries.push(GetEntry {
                    file: path.display().to_string(),
                    key: ".".to_string(),
                    value: JsonValue::Null,
                });
            }
            (Some(k), Some(fm_data), _) => {
                if let Some(value) = fm_data.get(k) {
                    entries.push(GetEntry {
                        file: path.display().to_string(),
                        key: k.to_string(),
                        value,
                    });
                }
            }
            _ => {}
        }
    }

    match output {
        OutputFormat::Files => {
            for entry in &entries {
                writeln!(out, "{}", entry.file)?;
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&entries)?)?;
        }
        OutputFormat::Plain => {
            for entry in &entries {
                let formatted = format_display_value(&entry.value);
                if single_explicit_file {
                    writeln!(out, "{formatted}")?;
                } else {
                    writeln!(out, "{}\t{formatted}", entry.file)?;
                }
            }
        }
        OutputFormat::Table => {
            if entries.is_empty() {
                return Ok(());
            }
            let col = key.unwrap_or("Frontmatter");
            let mut table = build_table(&["File", col]);
            for entry in &entries {
                table.add_row(vec![entry.file.clone(), format_display_value(&entry.value)]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}

fn execute_multi(
    keys: &[String],
    files: &[PathBuf],
    single_explicit_file: bool,
    filter: Option<&FilterExpr>,
    output: OutputFormat,
    out: &mut dyn Write,
) -> Result<()> {
    let mut entries = Vec::new();

    for path in files {
        let content = fs::read_to_string(path)
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

        let mut ordered = Vec::with_capacity(keys.len());
        let mut values = JsonMap::with_capacity(keys.len());
        let mut found_any = false;
        for k in keys {
            let val = fm.get(k);
            if val.is_some() {
                found_any = true;
            }
            values.insert(k.clone(), val.clone().unwrap_or(JsonValue::Null));
            ordered.push(val);
        }

        if found_any {
            entries.push(MultiGetEntry {
                file: path.display().to_string(),
                values,
                ordered,
            });
        }
    }

    match output {
        OutputFormat::Files => {
            for entry in &entries {
                writeln!(out, "{}", entry.file)?;
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&entries)?)?;
        }
        OutputFormat::Plain => {
            for entry in &entries {
                let cols: Vec<String> = entry
                    .ordered
                    .iter()
                    .map(|v| v.as_ref().map(format_display_value).unwrap_or_default())
                    .collect();
                if single_explicit_file {
                    writeln!(out, "{}", cols.join("\t"))?;
                } else {
                    writeln!(out, "{}\t{}", entry.file, cols.join("\t"))?;
                }
            }
        }
        OutputFormat::Table => {
            if entries.is_empty() {
                return Ok(());
            }
            let mut headers: Vec<&str> = Vec::with_capacity(keys.len() + 1);
            headers.push("File");
            headers.extend(keys.iter().map(String::as_str));
            let mut table = build_table(&headers);
            for entry in &entries {
                let mut row = Vec::with_capacity(keys.len() + 1);
                row.push(entry.file.clone());
                for val in &entry.ordered {
                    row.push(val.as_ref().map(format_display_value).unwrap_or_default());
                }
                table.add_row(row);
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
    fn parse_get_args_selectors_and_errors() {
        let (sel, paths) =
            parse_get_args(&[], &["title".to_string(), "docs/".to_string()]).unwrap();
        assert_eq!(sel, GetSelector::Single("title".to_string()));
        assert_eq!(paths, vec![PathBuf::from("docs/")]);

        let (sel, paths) =
            parse_get_args(&[], &["title, tags, title".to_string(), "a.md".to_string()]).unwrap();
        assert_eq!(
            sel,
            GetSelector::Multi(vec!["title".to_string(), "tags".to_string()])
        );
        assert_eq!(paths, vec![PathBuf::from("a.md")]);

        let (sel, paths) = parse_get_args(
            &["title".to_string(), "author.name".to_string()],
            &["docs/".to_string()],
        )
        .unwrap();
        assert_eq!(
            sel,
            GetSelector::Multi(vec!["title".to_string(), "author.name".to_string()])
        );
        assert_eq!(paths, vec![PathBuf::from("docs/")]);

        let (sel, paths) = parse_get_args(&[], &[".".to_string()]).unwrap();
        assert_eq!(sel, GetSelector::Root);
        assert!(paths.is_empty());

        assert!(parse_get_args(&[], &[]).is_err());
        assert!(parse_get_args(&[], &["title,,tags".to_string()]).is_err());
        assert!(parse_get_args(&[], &[".,title".to_string()]).is_err());
    }

    #[test]
    fn get_multi_plain_empty_table_and_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        let c = dir.path().join("c_body_only.md");
        fs::write(&a, "---\ntitle: One\ntags:\n  - rust\n---\nBody\n").unwrap();
        fs::write(&b, "---\ntitle: Two\n---\nBody\n").unwrap();
        fs::write(&c, "# Body only\n").unwrap();

        let dir_arg = dir.path().to_str().unwrap().to_string();

        // Multiple files in plain format emit `<file>\t<value>`.
        let cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[],
            &["title".to_string(), dir_arg.clone()],
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("\tOne\n"));
        assert!(text.contains("\tTwo\n"));

        // Multi-key query in JSON, plain, table, and files modes (b.md missing tags -> null / "").
        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[],
            &["title,tags".to_string(), dir_arg.clone()],
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["values"]["title"], json!("One"));
        assert_eq!(parsed[0]["values"]["tags"], json!(["rust"]));
        assert_eq!(parsed[1]["values"]["title"], json!("Two"));
        assert_eq!(parsed[1]["values"]["tags"], json!(null));

        // Multi-key single explicit file in plain mode emits tab-delimited values without file column.
        let mut out = Vec::new();
        execute(
            &["title".to_string(), "tags".to_string()],
            &[a.to_str().unwrap().to_string()],
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "One\t[\"rust\"]");

        // Root `.` dump in JSON returns full frontmatter object and skips body-only files by default.
        let mut out = Vec::new();
        execute(
            &[],
            &[".".to_string(), dir_arg.clone()],
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let root_parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(root_parsed.len(), 2);
        assert_eq!(root_parsed[0]["key"], json!("."));
        assert_eq!(root_parsed[0]["value"]["title"], json!("One"));

        // Root `.` with `not has_frontmatter` in files mode lists body-only file.
        let no_fm = FilterExpr::parse("not has_frontmatter").unwrap();
        let files_cfg = ResolvedConfig {
            output: OutputFormat::Files,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[],
            &[".".to_string(), dir_arg.clone()],
            Some(&no_fm),
            &files_cfg,
            &mut out,
        )
        .unwrap();
        let files_out = String::from_utf8(out).unwrap();
        assert!(files_out.trim().ends_with("c_body_only.md"));

        // Empty table output prints nothing.
        let table_cfg = ResolvedConfig {
            output: OutputFormat::Table,
            ..ResolvedConfig::default()
        };
        let mut empty_out = Vec::new();
        execute(
            &[],
            &["nonexistent".to_string(), dir_arg],
            None,
            &table_cfg,
            &mut empty_out,
        )
        .unwrap();
        assert!(empty_out.is_empty());

        // Invalid frontmatter returns error.
        let bad = dir.path().join("bad.md");
        fs::write(&bad, "---\n[unclosed\n---\nBody\n").unwrap();
        let mut err_out = Vec::new();
        assert!(execute(
            &[],
            &["title".to_string(), bad.to_str().unwrap().to_string()],
            None,
            &cfg,
            &mut err_out
        )
        .is_err());
    }
}
