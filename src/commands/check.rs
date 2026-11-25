use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cli::OutputFormat;
use crate::commands::{build_table, collect_markdown_files};
use crate::config::ResolvedConfig;
use crate::frontmatter;
use crate::frontmatter::filter::FilterExpr;

#[derive(Debug, Serialize)]
struct CheckEntry {
    file: String,
    valid: bool,
    errors: Vec<String>,
}

pub fn execute(
    paths: &[PathBuf],
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let schema_path = config.schema.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "No JSON Schema specified. Pass --schema <PATH>, set MDFM_SCHEMA, or configure 'schema' in .mdfm.toml"
        )
    })?;

    let schema_raw = fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema file '{}'", schema_path.display()))?;
    let schema_json: JsonValue = serde_json::from_str(&schema_raw)
        .with_context(|| format!("Failed to parse JSON Schema '{}'", schema_path.display()))?;
    let validator = jsonschema::validator_for(&schema_json)
        .map_err(|e| anyhow::anyhow!("Invalid JSON Schema '{}': {e}", schema_path.display()))?;

    let files = collect_markdown_files(paths, &config.ignore)?;
    let mut results = Vec::new();

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&content);

        let parsed = match doc.parse_frontmatter() {
            Ok(fm) => fm,
            Err(err) => {
                results.push(CheckEntry {
                    file: path.display().to_string(),
                    valid: false,
                    errors: vec![err.to_string()],
                });
                continue;
            }
        };

        let instance = match parsed {
            Some(ref fm) => match fm.to_json_value() {
                Ok(v) => v,
                Err(err) => {
                    results.push(CheckEntry {
                        file: path.display().to_string(),
                        valid: false,
                        errors: vec![err.to_string()],
                    });
                    continue;
                }
            },
            None => {
                if filter.is_some() {
                    continue;
                }
                json!({})
            }
        };

        if let Some(expr) = filter {
            if !expr.matches(&instance) {
                continue;
            }
        }

        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| {
                let loc = e.instance_path.to_string();
                if loc.is_empty() {
                    e.to_string()
                } else {
                    format!("{loc}: {e}")
                }
            })
            .collect();

        results.push(CheckEntry {
            file: path.display().to_string(),
            valid: errors.is_empty(),
            errors,
        });
    }

    let failed_count = results.iter().filter(|r| !r.valid).count();

    match config.output {
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&results)?)?;
        }
        OutputFormat::Plain => {
            for r in &results {
                if r.valid {
                    writeln!(out, "{}\tok", r.file)?;
                } else {
                    for err in &r.errors {
                        writeln!(out, "{}\terror: {err}", r.file)?;
                    }
                }
            }
        }
        OutputFormat::Table => {
            if !results.is_empty() {
                let mut table = build_table(&["File", "Status", "Errors"]);
                for r in &results {
                    let status = if r.valid {
                        format!("{}", "ok".if_supports_color(Stream::Stdout, |t| t.green()))
                    } else {
                        format!("{}", "fail".if_supports_color(Stream::Stdout, |t| t.red()))
                    };
                    table.add_row(vec![r.file.clone(), status, r.errors.join("; ")]);
                }
                writeln!(out, "{table}")?;
            }
        }
    }

    if failed_count > 0 {
        anyhow::bail!("Frontmatter validation failed for {failed_count} file(s)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_errors_filters_body_only_and_table_output() {
        let dir = tempfile::tempdir().unwrap();
        let valid = dir.path().join("valid.md");
        let invalid = dir.path().join("invalid.md");
        let bad_fm = dir.path().join("bad_fm.md");
        let plain = dir.path().join("plain.md");
        fs::write(&valid, "---\ntitle: Ok\ndraft: false\n---\nBody\n").unwrap();
        fs::write(&invalid, "---\ntitle: \"\"\ndraft: \"yes\"\n---\nBody\n").unwrap();
        fs::write(&bad_fm, "---\n[broken\n---\nBody\n").unwrap();
        fs::write(&plain, "# Body only\n").unwrap();

        // Missing schema config errors.
        let no_schema_cfg = ResolvedConfig::default();
        let mut out = Vec::new();
        assert!(execute(std::slice::from_ref(&valid), None, &no_schema_cfg, &mut out).is_err());

        // Nonexistent schema file errors.
        let missing_schema_cfg = ResolvedConfig {
            schema: Some(dir.path().join("missing.json")),
            ..ResolvedConfig::default()
        };
        assert!(execute(
            std::slice::from_ref(&valid),
            None,
            &missing_schema_cfg,
            &mut out
        )
        .is_err());

        // Invalid JSON in schema file errors.
        let bad_json_schema = dir.path().join("bad_json.json");
        fs::write(&bad_json_schema, "{not json").unwrap();
        let bad_json_cfg = ResolvedConfig {
            schema: Some(bad_json_schema),
            ..ResolvedConfig::default()
        };
        assert!(execute(std::slice::from_ref(&valid), None, &bad_json_cfg, &mut out).is_err());

        // Invalid JSON Schema specification errors.
        let bad_spec_schema = dir.path().join("bad_spec.json");
        fs::write(&bad_spec_schema, r#"{"type": 123}"#).unwrap();
        let bad_spec_cfg = ResolvedConfig {
            schema: Some(bad_spec_schema),
            ..ResolvedConfig::default()
        };
        assert!(execute(std::slice::from_ref(&valid), None, &bad_spec_cfg, &mut out).is_err());

        // Valid schema file.
        let schema = dir.path().join("schema.json");
        fs::write(
            &schema,
            r#"{"type":"object","required":["title","draft"],"properties":{"title":{"type":"string","minLength":1},"draft":{"type":"boolean"}}}"#,
        )
        .unwrap();

        // Table output covering valid, invalid, bad frontmatter, and body-only ({}).
        let table_cfg = ResolvedConfig {
            output: OutputFormat::Table,
            schema: Some(schema.clone()),
            ..ResolvedConfig::default()
        };
        let mut table_out = Vec::new();
        let res = execute(
            &[
                valid.clone(),
                invalid.clone(),
                bad_fm.clone(),
                plain.clone(),
            ],
            None,
            &table_cfg,
            &mut table_out,
        );
        assert!(res.is_err());
        let table_str = String::from_utf8(table_out).unwrap();
        assert!(table_str.contains("ok"));
        assert!(table_str.contains("fail"));
        assert!(table_str.contains("bad_fm.md"));

        // --where filter skips body-only and non-matching files, passes on valid file.
        let filter = FilterExpr::parse("draft == false").unwrap();
        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            schema: Some(schema),
            ..ResolvedConfig::default()
        };
        let mut json_out = Vec::new();
        execute(
            &[valid, invalid, plain],
            Some(&filter),
            &json_cfg,
            &mut json_out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&json_out).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["valid"], json!(true));
    }
}
