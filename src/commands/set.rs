use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::{
    build_table, collect_markdown_files, frontmatter_prefix, render_unified_diff,
};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::{FilterContext, FilterExpr};
use crate::frontmatter::{self, Format, FrontmatterData};

#[derive(Debug, Serialize)]
struct SetEntry {
    file: String,
    changed: bool,
    dry_run: bool,
    created_frontmatter: bool,
    format: Format,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

/// Parse a raw CLI value string into a typed JSON value:
/// booleans, integers, finite floats, JSON arrays/objects/quoted strings, falling back to string.
pub fn parse_cli_value(raw: &str) -> JsonValue {
    match raw {
        "true" => return JsonValue::Bool(true),
        "false" => return JsonValue::Bool(false),
        "null" => return JsonValue::Null,
        _ => {}
    }

    if let Ok(i) = raw.parse::<i64>() {
        return JsonValue::Number(i.into());
    }

    if !raw.eq_ignore_ascii_case("nan")
        && !raw.eq_ignore_ascii_case("inf")
        && !raw.eq_ignore_ascii_case("+inf")
        && !raw.eq_ignore_ascii_case("-inf")
        && !raw.eq_ignore_ascii_case("infinity")
        && !raw.eq_ignore_ascii_case("+infinity")
        && !raw.eq_ignore_ascii_case("-infinity")
    {
        if let Ok(f) = raw.parse::<f64>() {
            if f.is_finite() {
                if let Some(num) = serde_json::Number::from_f64(f) {
                    return JsonValue::Number(num);
                }
            }
        }
    }

    if raw.starts_with('[') || raw.starts_with('{') || raw.starts_with('"') {
        if let Ok(parsed) = serde_json::from_str::<JsonValue>(raw) {
            return parsed;
        }
    }

    JsonValue::String(raw.to_string())
}

/// Mutation operation supported by `mdfm set`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOp {
    /// Replace or insert key (`KEY=VALUE`).
    Assign,
    /// Append item(s) to an array with deduplication (`KEY+=VALUE`).
    Append,
    /// Remove matching item(s) from an array (`KEY-=VALUE`).
    Remove,
}

/// A parsed `(key, op, typed_value)` mutation for `mdfm set`.
#[derive(Debug, Clone, PartialEq)]
pub struct SetAssignment {
    pub key: String,
    pub op: SetOp,
    pub value: JsonValue,
}

/// Partition positional `args` into parsed mutations (`KEY=VALUE`, `KEY+=VALUE`, `KEY-=VALUE`)
/// and target file/dir paths.
pub fn parse_set_args(args: &[String]) -> Result<(Vec<SetAssignment>, Vec<PathBuf>)> {
    let mut assignments = Vec::new();
    let mut paths = Vec::new();
    let mut in_paths = false;

    for arg in args {
        if !in_paths {
            if let Some((lhs, raw_val)) = arg.split_once('=') {
                if !Path::new(arg).exists() {
                    let (key, op) = if let Some(k) = lhs.strip_suffix('+') {
                        (k, SetOp::Append)
                    } else if let Some(k) = lhs.strip_suffix('-') {
                        (k, SetOp::Remove)
                    } else {
                        (lhs, SetOp::Assign)
                    };
                    if key.is_empty() {
                        anyhow::bail!("Invalid assignment '{arg}': key cannot be empty");
                    }
                    assignments.push(SetAssignment {
                        key: key.to_string(),
                        op,
                        value: parse_cli_value(raw_val),
                    });
                    continue;
                }
            }
            in_paths = true;
        }
        paths.push(PathBuf::from(arg));
    }

    if assignments.is_empty() {
        anyhow::bail!("Expected at least one KEY=VALUE, KEY+=VALUE, or KEY-=VALUE assignment");
    }

    Ok((assignments, paths))
}

pub fn execute(
    args: &[String],
    dry_run: bool,
    show_diff: bool,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let (assignments, paths) = parse_set_args(args)?;
    let files = collect_markdown_files(&paths, &config.ignore, &config.extensions)?;
    let mut results = Vec::new();

    for path in files {
        let original = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&original);
        let existing_fm = doc
            .parse_frontmatter()
            .with_context(|| format!("Failed to parse frontmatter in '{}'", path.display()))?;

        let before_json = existing_fm
            .as_ref()
            .map(FrontmatterData::to_json_value)
            .transpose()?;

        if let Some(expr) = filter {
            let ctx = FilterContext {
                frontmatter: before_json.as_ref(),
                format: existing_fm.as_ref().map(FrontmatterData::format),
            };
            if !expr.matches_doc(ctx) {
                continue;
            }
        }

        let had_frontmatter = existing_fm.is_some();
        let mut fm = existing_fm.unwrap_or_else(|| FrontmatterData::empty(config.default_format));

        for assignment in &assignments {
            match assignment.op {
                SetOp::Assign => fm
                    .set(&assignment.key, assignment.value.clone())
                    .with_context(|| {
                        format!(
                            "Failed to set key '{}' in '{}'",
                            assignment.key,
                            path.display()
                        )
                    })?,
                SetOp::Append => fm
                    .append(&assignment.key, assignment.value.clone())
                    .with_context(|| {
                        format!(
                            "Failed to append to key '{}' in '{}'",
                            assignment.key,
                            path.display()
                        )
                    })?,
                SetOp::Remove => fm
                    .remove_from_array(&assignment.key, assignment.value.clone())
                    .with_context(|| {
                        format!(
                            "Failed to remove from key '{}' in '{}'",
                            assignment.key,
                            path.display()
                        )
                    })?,
            }
        }

        let after_json = fm.to_json_value()?;
        let created_frontmatter =
            !had_frontmatter && after_json.as_object().is_some_and(|m| !m.is_empty());

        let semantically_changed = match before_json {
            None => created_frontmatter,
            Some(before) => after_json != before,
        };

        let (changed, rendered_opt) = if semantically_changed {
            let rendered = doc.render_with_options(Some(&fm), config.yaml_format_options())?;
            let bytes_changed = rendered != original;
            if bytes_changed && !dry_run {
                fs::write(&path, &rendered)
                    .with_context(|| format!("Failed to write '{}'", path.display()))?;
            }
            (bytes_changed, bytes_changed.then_some(rendered))
        } else {
            (false, None)
        };

        let file_str = path.display().to_string();
        let diff_text = if show_diff {
            rendered_opt.as_deref().map(|rendered| {
                let before_fm = frontmatter_prefix(&original, doc.body.len());
                let after_fm = frontmatter_prefix(rendered, doc.body.len());
                let use_color = !matches!(config.output, OutputFormat::Json);
                render_unified_diff(&file_str, before_fm, after_fm, use_color)
            })
        } else {
            None
        };

        results.push(SetEntry {
            file: file_str,
            changed,
            dry_run,
            created_frontmatter,
            format: fm.format(),
            diff: diff_text,
        });
    }

    match config.output {
        OutputFormat::Files => {
            for r in &results {
                writeln!(out, "{}", r.file)?;
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&results)?)?;
        }
        OutputFormat::Plain | OutputFormat::Table if show_diff => {
            for r in &results {
                if let Some(ref d) = r.diff {
                    if !d.is_empty() {
                        write!(out, "{d}")?;
                    }
                }
            }
        }
        OutputFormat::Plain => {
            for r in &results {
                let status = status_label(r);
                writeln!(out, "{}\t{status}", r.file)?;
            }
        }
        OutputFormat::Table => {
            if results.is_empty() {
                return Ok(());
            }
            let mut table = build_table(&["File", "Status", "Format"]);
            for r in &results {
                let label = status_label(r);
                let colored = if r.changed {
                    if r.dry_run {
                        format!(
                            "{}",
                            label.if_supports_color(Stream::Stdout, |t| t.yellow())
                        )
                    } else {
                        format!("{}", label.if_supports_color(Stream::Stdout, |t| t.green()))
                    }
                } else {
                    format!(
                        "{}",
                        label.if_supports_color(Stream::Stdout, |t| t.dimmed())
                    )
                };
                table.add_row(vec![r.file.clone(), colored, r.format.to_string()]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}

fn status_label(entry: &SetEntry) -> &'static str {
    if !entry.changed {
        "unchanged"
    } else if entry.dry_run {
        if entry.created_frontmatter {
            "would create (dry-run)"
        } else {
            "would update (dry-run)"
        }
    } else if entry.created_frontmatter {
        "created"
    } else {
        "updated"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_cli_values_all_types() {
        assert_eq!(parse_cli_value("true"), json!(true));
        assert_eq!(parse_cli_value("false"), json!(false));
        assert_eq!(parse_cli_value("null"), json!(null));
        assert_eq!(parse_cli_value("42"), json!(42));
        assert_eq!(parse_cli_value("-7"), json!(-7));
        assert_eq!(parse_cli_value("2.5"), json!(2.5));
        assert_eq!(parse_cli_value(r#"["rust","cli"]"#), json!(["rust", "cli"]));
        assert_eq!(parse_cli_value(r#"{"a":1}"#), json!({"a": 1}));
        assert_eq!(parse_cli_value(r#""1.0""#), json!("1.0"));
        assert_eq!(parse_cli_value("Hello World"), json!("Hello World"));
        assert_eq!(parse_cli_value("inf"), json!("inf"));
    }

    #[test]
    fn parse_set_args_splits_assignments_and_paths() {
        let args = vec![
            "draft=false".to_string(),
            "author.name=Nick".to_string(),
            "tags+=must-try".to_string(),
            "tags-=draft".to_string(),
            r#"tags+=["a","b"]"#.to_string(),
            "posts/hello.md".to_string(),
        ];
        let (assignments, paths) = parse_set_args(&args).unwrap();
        assert_eq!(assignments.len(), 5);
        assert_eq!(
            assignments[0],
            SetAssignment {
                key: "draft".to_string(),
                op: SetOp::Assign,
                value: json!(false),
            }
        );
        assert_eq!(
            assignments[1],
            SetAssignment {
                key: "author.name".to_string(),
                op: SetOp::Assign,
                value: json!("Nick"),
            }
        );
        assert_eq!(
            assignments[2],
            SetAssignment {
                key: "tags".to_string(),
                op: SetOp::Append,
                value: json!("must-try"),
            }
        );
        assert_eq!(
            assignments[3],
            SetAssignment {
                key: "tags".to_string(),
                op: SetOp::Remove,
                value: json!("draft"),
            }
        );
        assert_eq!(
            assignments[4],
            SetAssignment {
                key: "tags".to_string(),
                op: SetOp::Append,
                value: json!(["a", "b"]),
            }
        );
        assert_eq!(paths, vec![PathBuf::from("posts/hello.md")]);

        // Error cases: empty key (including += / -=) and no assignments.
        assert!(parse_set_args(&["=value".to_string()]).is_err());
        assert!(parse_set_args(&["+=value".to_string()]).is_err());
        assert!(parse_set_args(&["-=value".to_string()]).is_err());
        assert!(parse_set_args(&["posts/hello.md".to_string()]).is_err());

        // Existing path containing '=' is treated as a file path, not an assignment.
        let dir = tempfile::tempdir().unwrap();
        let eq_file = dir.path().join("a=b.md");
        fs::write(&eq_file, "---\ntitle: Eq\n---\n").unwrap();
        let (a2, p2) = parse_set_args(&[
            "draft=true".to_string(),
            eq_file.to_str().unwrap().to_string(),
        ])
        .unwrap();
        assert_eq!(a2.len(), 1);
        assert_eq!(p2, vec![eq_file]);
    }

    #[test]
    fn set_execute_table_filters_dry_run_unchanged_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let post = dir.path().join("post.md");
        let plain = dir.path().join("plain.md");
        fs::write(&post, "---\ntitle: Hello\ndraft: true\n---\nBody\n").unwrap();
        fs::write(&plain, "# Body only\n").unwrap();

        let table_cfg = ResolvedConfig {
            output: OutputFormat::Table,
            ..ResolvedConfig::default()
        };

        // Dry-run creating frontmatter on body-only file in table format.
        let mut out = Vec::new();
        execute(
            &[
                "title=Created".to_string(),
                plain.to_str().unwrap().to_string(),
            ],
            true,
            false,
            None,
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("would create (dry-run)"));

        // Unchanged set in table format.
        let mut out = Vec::new();
        execute(
            &["draft=true".to_string(), post.to_str().unwrap().to_string()],
            false,
            false,
            None,
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("unchanged"));

        // --where filter skips body-only file and non-matching file -> empty table output.
        let filter = FilterExpr::parse("draft == false").unwrap();
        let mut out = Vec::new();
        execute(
            &[
                "draft=false".to_string(),
                dir.path().to_str().unwrap().to_string(),
            ],
            false,
            false,
            Some(&filter),
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(out.is_empty());

        // --where filter matching file updates in dry-run table mode.
        let filter_match = FilterExpr::parse("draft == true").unwrap();
        let mut out = Vec::new();
        execute(
            &[
                "draft=false".to_string(),
                dir.path().to_str().unwrap().to_string(),
            ],
            true,
            false,
            Some(&filter_match),
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("would update (dry-run)"));

        // Traversing scalar key returns error.
        let mut err_out = Vec::new();
        assert!(execute(
            &[
                "title.sub=1".to_string(),
                post.to_str().unwrap().to_string()
            ],
            false,
            false,
            None,
            &table_cfg,
            &mut err_out
        )
        .is_err());

        // Malformed frontmatter returns parse error.
        let bad = dir.path().join("bad.md");
        fs::write(&bad, "---\n[broken\n---\n").unwrap();
        assert!(execute(
            &["draft=false".to_string(), bad.to_str().unwrap().to_string()],
            false,
            false,
            None,
            &table_cfg,
            &mut err_out
        )
        .is_err());
    }

    #[test]
    fn set_skips_write_when_semantically_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_file = dir.path().join("recipe.md");
        // Indented YAML sequence + blank line inside frontmatter that serde_yaml_ng would reformat.
        let yaml_orig = "\
---
title: Savory Star Bread
comments: true

tags:
  - bread
  - must-try
---

# Savory Star Bread
";
        fs::write(&yaml_file, yaml_orig).unwrap();

        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };

        // Dry-run and write mode both report unchanged and keep original YAML formatting intact.
        for dry_run in [true, false] {
            let mut out = Vec::new();
            execute(
                &[
                    "comments=true".to_string(),
                    yaml_file.to_str().unwrap().to_string(),
                ],
                dry_run,
                false,
                None,
                &json_cfg,
                &mut out,
            )
            .unwrap();

            let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0]["changed"], json!(false));
            assert_eq!(fs::read_to_string(&yaml_file).unwrap(), yaml_orig);
        }

        // Changing the semantic value updates the file.
        let mut out = Vec::new();
        execute(
            &[
                "comments=false".to_string(),
                yaml_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(true));
        let updated = fs::read_to_string(&yaml_file).unwrap();
        assert!(updated.contains("comments: false"));

        // TOML semantic no-op also skips writing and preserves comments/formatting verbatim.
        let toml_file = dir.path().join("note.md");
        let toml_orig = "+++\n# header\ncomments = true  # inline\n+++\n\n# Body\n";
        fs::write(&toml_file, toml_orig).unwrap();

        let mut out = Vec::new();
        execute(
            &[
                "comments=true".to_string(),
                toml_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(false));
        assert_eq!(fs::read_to_string(&toml_file).unwrap(), toml_orig);
    }

    #[test]
    fn set_array_append_remove_and_body_only_noop() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_file = dir.path().join("recipe.md");
        let yaml_orig = "\
---
title: Star Bread
tags:
  - bread
  - draft
---

# Star Bread
";
        fs::write(&yaml_file, yaml_orig).unwrap();

        let plain_file = dir.path().join("plain.md");
        let plain_orig = "# Body Only\n";
        fs::write(&plain_file, plain_orig).unwrap();

        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };

        // Removing from a body-only file does not create empty frontmatter.
        let mut out = Vec::new();
        execute(
            &[
                "tags-=draft".to_string(),
                plain_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(false));
        assert_eq!(parsed[0]["created_frontmatter"], json!(false));
        assert_eq!(fs::read_to_string(&plain_file).unwrap(), plain_orig);

        // Appending an already-present tag is a semantic no-op (formatting preserved).
        let mut out = Vec::new();
        execute(
            &[
                "tags+=bread".to_string(),
                yaml_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(false));
        assert_eq!(fs::read_to_string(&yaml_file).unwrap(), yaml_orig);

        // Combined append + remove in one invocation.
        let mut out = Vec::new();
        execute(
            &[
                "tags+=must-try".to_string(),
                "tags-=draft".to_string(),
                yaml_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(true));
        let updated = fs::read_to_string(&yaml_file).unwrap();
        let fm = frontmatter::split(&updated)
            .parse_frontmatter()
            .unwrap()
            .unwrap();
        assert_eq!(fm.get("tags"), Some(json!(["bread", "must-try"])));

        // Appending to a body-only file creates frontmatter with the new array.
        let mut out = Vec::new();
        execute(
            &[
                "tags+=must-try".to_string(),
                plain_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(true));
        assert_eq!(parsed[0]["created_frontmatter"], json!(true));
        let plain_updated = fs::read_to_string(&plain_file).unwrap();
        assert!(plain_updated.starts_with("---\ntags:\n  - must-try\n---\n\n# Body Only\n"));

        // Non-array key returns error on += and -=.
        let mut err_out = Vec::new();
        assert!(execute(
            &[
                "title+=oops".to_string(),
                yaml_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut err_out,
        )
        .is_err());
        assert!(execute(
            &[
                "title-=oops".to_string(),
                yaml_file.to_str().unwrap().to_string(),
            ],
            false,
            false,
            None,
            &json_cfg,
            &mut err_out,
        )
        .is_err());
    }
}
