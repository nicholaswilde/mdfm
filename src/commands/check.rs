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
