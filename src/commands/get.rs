use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cli::OutputFormat;
use crate::commands::{build_table, collect_markdown_files, format_display_value};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::FilterExpr;
use crate::frontmatter::{self};

#[derive(Debug, Serialize)]
struct GetEntry {
    file: String,
    key: String,
    value: JsonValue,
}

pub fn execute(
    key: &str,
    paths: &[PathBuf],
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let single_explicit_file = paths.len() == 1 && paths[0].is_file();
    let files = collect_markdown_files(paths, &config.ignore)?;
    let mut entries = Vec::new();

    for path in files {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&content);
        let fm = doc
            .parse_frontmatter()
            .with_context(|| format!("Failed to parse frontmatter in '{}'", path.display()))?;

        let Some(fm) = fm else {
            continue;
        };

        if let Some(expr) = filter {
            let json_obj = fm.to_json_value()?;
            if !expr.matches(&json_obj) {
                continue;
            }
        }

        if let Some(value) = fm.get(key) {
            entries.push(GetEntry {
                file: path.display().to_string(),
                key: key.to_string(),
                value,
            });
        }
    }

    match config.output {
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
            let mut table = build_table(&["File", key]);
            for entry in &entries {
                table.add_row(vec![entry.file.clone(), format_display_value(&entry.value)]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}
