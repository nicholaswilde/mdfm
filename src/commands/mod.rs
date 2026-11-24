pub mod check;
pub mod del;
pub mod get;
pub mod set;

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

use crate::frontmatter::filter::FilterExpr;

/// Parse an optional `--where` filter expression.
pub fn compile_filter(expr: Option<&str>) -> Result<Option<FilterExpr>> {
    match expr {
        Some(raw) => FilterExpr::parse(raw)
            .map(Some)
            .with_context(|| format!("Invalid --where expression: '{raw}'")),
        None => Ok(None),
    }
}

/// Collect Markdown files (`.md`, `.markdown`) from the given paths, respecting `.gitignore`
/// and any configured `ignore` glob patterns. Defaults to `.` when `paths` is empty.
pub fn collect_markdown_files(
    paths: &[PathBuf],
    ignore_patterns: &[String],
) -> Result<Vec<PathBuf>> {
    let targets: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };

    let mut files = Vec::new();

    for target in &targets {
        if !target.exists() {
            anyhow::bail!("Path does not exist: '{}'", target.display());
        }

        if target.is_file() {
            files.push(target.clone());
            continue;
        }

        let mut builder = WalkBuilder::new(target);
        builder.require_git(false);

        if !ignore_patterns.is_empty() {
            let mut overrides = OverrideBuilder::new(target);
            for pat in ignore_patterns {
                let rule = if pat.starts_with('!') {
                    pat.clone()
                } else {
                    format!("!{pat}")
                };
                overrides
                    .add(&rule)
                    .with_context(|| format!("Invalid ignore glob pattern '{pat}'"))?;
            }
            let compiled = overrides
                .build()
                .context("Failed to compile ignore patterns")?;
            builder.overrides(compiled);
        }

        for entry in builder.build() {
            let entry = entry.with_context(|| {
                format!("Failed while traversing directory '{}'", target.display())
            })?;
            if entry.file_type().is_some_and(|ft| ft.is_file()) && is_markdown_path(entry.path()) {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

pub(crate) fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

/// Render a JSON value for human-friendly table or plain output.
pub(crate) fn format_display_value(val: &JsonValue) -> String {
    match val {
        JsonValue::String(s) => s.clone(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// Build a consistently styled `comfy_table::Table` with headers.
pub(crate) fn build_table(headers: &[&str]) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers);
    table
}
