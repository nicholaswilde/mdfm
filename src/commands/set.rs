use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::{build_table, collect_markdown_files};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::FilterExpr;
use crate::frontmatter::{self, Format, FrontmatterData};

#[derive(Debug, Serialize)]
struct SetEntry {
    file: String,
    changed: bool,
    dry_run: bool,
    created_frontmatter: bool,
    format: Format,
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

/// A parsed `(key_path, typed_value)` pair for `mdfm set`.
pub type SetAssignment = (String, JsonValue);

/// Partition positional `args` into `(key, parsed_value)` assignments and target file/dir paths.
pub fn parse_set_args(args: &[String]) -> Result<(Vec<SetAssignment>, Vec<PathBuf>)> {
    let mut assignments = Vec::new();
    let mut paths = Vec::new();
    let mut in_paths = false;

    for arg in args {
        if !in_paths {
            if let Some((key, raw_val)) = arg.split_once('=') {
                if !Path::new(arg).exists() {
                    if key.is_empty() {
                        anyhow::bail!("Invalid assignment '{arg}': key cannot be empty");
                    }
                    assignments.push((key.to_string(), parse_cli_value(raw_val)));
                    continue;
                }
            }
            in_paths = true;
        }
        paths.push(PathBuf::from(arg));
    }

    if assignments.is_empty() {
        anyhow::bail!("Expected at least one KEY=VALUE assignment");
    }

    Ok((assignments, paths))
}

pub fn execute(
    args: &[String],
    dry_run: bool,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let (assignments, paths) = parse_set_args(args)?;
    let files = collect_markdown_files(&paths, &config.ignore)?;
    let mut results = Vec::new();

    for path in files {
        let original = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&original);
        let existing_fm = doc
            .parse_frontmatter()
            .with_context(|| format!("Failed to parse frontmatter in '{}'", path.display()))?;

        if let Some(expr) = filter {
            let Some(ref fm) = existing_fm else {
                continue;
            };
            let json_obj = fm.to_json_value()?;
            if !expr.matches(&json_obj) {
                continue;
            }
        }

        let created_frontmatter = existing_fm.is_none();
        let mut fm = existing_fm.unwrap_or_else(|| FrontmatterData::empty(config.default_format));

        for (key, value) in &assignments {
            fm.set(key, value.clone())
                .with_context(|| format!("Failed to set key '{key}' in '{}'", path.display()))?;
        }

        let rendered = doc.render(Some(&fm))?;
        let changed = rendered != original;

        if changed && !dry_run {
            fs::write(&path, &rendered)
                .with_context(|| format!("Failed to write '{}'", path.display()))?;
        }

        results.push(SetEntry {
            file: path.display().to_string(),
            changed,
            dry_run,
            created_frontmatter,
            format: fm.format(),
        });
    }

    match config.output {
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&results)?)?;
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
            "posts/hello.md".to_string(),
        ];
        let (assignments, paths) = parse_set_args(&args).unwrap();
        assert_eq!(assignments.len(), 2);
        assert_eq!(assignments[0], ("draft".to_string(), json!(false)));
        assert_eq!(assignments[1], ("author.name".to_string(), json!("Nick")));
        assert_eq!(paths, vec![PathBuf::from("posts/hello.md")]);

        // Error cases: empty key and no assignments.
        assert!(parse_set_args(&["=value".to_string()]).is_err());
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
            None,
            &table_cfg,
            &mut err_out
        )
        .is_err());
    }
}
