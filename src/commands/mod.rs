pub mod audit;
pub mod check;
pub mod del;
pub mod fmt;
pub mod get;
pub mod mv;
pub mod set;
pub mod update;

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use owo_colors::{OwoColorize, Stream};
use serde_json::Value as JsonValue;
use std::fmt::Write as FmtWrite;
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

/// Collect files matching `extensions` (default: `.md`, `.markdown`) from the given paths,
/// respecting `.gitignore` and any configured `ignore` glob patterns. Defaults to `.` when
/// `paths` is empty.
pub fn collect_markdown_files(
    paths: &[PathBuf],
    ignore_patterns: &[String],
    extensions: &[String],
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
            if entry.file_type().is_some_and(|ft| ft.is_file())
                && has_matching_extension(entry.path(), extensions)
            {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

pub(crate) fn has_matching_extension(path: &Path, extensions: &[String]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    extensions.iter().any(|allowed| {
        let normalized = allowed.trim().trim_start_matches('.');
        !normalized.is_empty() && ext.eq_ignore_ascii_case(normalized)
    })
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

/// Extract the frontmatter prefix of a document (everything before the verbatim body slice).
pub(crate) fn frontmatter_prefix(full: &str, body_len: usize) -> &str {
    let prefix_len = full.len().saturating_sub(body_len);
    &full[..prefix_len]
}

fn split_diff_lines(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    let trimmed = s.strip_suffix('\n').unwrap_or(s);
    trimmed
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffOp<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

/// Compute a unified diff between two frontmatter text slices (3 lines of context).
/// Returns an empty string when `before` and `after` have identical lines.
pub(crate) fn render_unified_diff(file: &str, before: &str, after: &str, color: bool) -> String {
    let old_lines = split_diff_lines(before);
    let new_lines = split_diff_lines(after);
    if old_lines == new_lines {
        return String::new();
    }

    let n = old_lines.len();
    let m = new_lines.len();
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if old_lines[i] == new_lines[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut ops = Vec::with_capacity(n + m);
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old_lines[i] == new_lines[j] {
            ops.push(DiffOp::Equal(old_lines[i]));
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            ops.push(DiffOp::Delete(old_lines[i]));
            i += 1;
        } else {
            ops.push(DiffOp::Insert(new_lines[j]));
            j += 1;
        }
    }
    while i < n {
        ops.push(DiffOp::Delete(old_lines[i]));
        i += 1;
    }
    while j < m {
        ops.push(DiffOp::Insert(new_lines[j]));
        j += 1;
    }

    // Track 0-based (old_idx, new_idx) before each op for accurate hunk headers.
    let mut positions = Vec::with_capacity(ops.len());
    let (mut old_pos, mut new_pos) = (0usize, 0usize);
    for op in &ops {
        positions.push((old_pos, new_pos));
        match op {
            DiffOp::Equal(_) => {
                old_pos += 1;
                new_pos += 1;
            }
            DiffOp::Delete(_) => old_pos += 1,
            DiffOp::Insert(_) => new_pos += 1,
        }
    }

    const CONTEXT: usize = 3;
    let changed_indices: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter_map(|(idx, op)| (!matches!(op, DiffOp::Equal(_))).then_some(idx))
        .collect();
    if changed_indices.is_empty() {
        return String::new();
    }

    let mut hunks: Vec<(usize, usize)> = Vec::new();
    for idx in changed_indices {
        let start = idx.saturating_sub(CONTEXT);
        let end = (idx + CONTEXT + 1).min(ops.len());
        if let Some(last) = hunks.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        hunks.push((start, end));
    }

    let mut out = String::new();
    let header_old = format!("--- {file}");
    let header_new = format!("+++ {file}");
    if color {
        let _ = writeln!(
            out,
            "{}",
            header_old.if_supports_color(Stream::Stdout, |t| t.bold())
        );
        let _ = writeln!(
            out,
            "{}",
            header_new.if_supports_color(Stream::Stdout, |t| t.bold())
        );
    } else {
        let _ = writeln!(out, "{header_old}");
        let _ = writeln!(out, "{header_new}");
    }

    for (start, end) in hunks {
        let (old_base, new_base) = positions[start];
        let old_count = ops[start..end]
            .iter()
            .filter(|op| matches!(op, DiffOp::Equal(_) | DiffOp::Delete(_)))
            .count();
        let new_count = ops[start..end]
            .iter()
            .filter(|op| matches!(op, DiffOp::Equal(_) | DiffOp::Insert(_)))
            .count();
        let old_start = if old_count == 0 {
            old_base
        } else {
            old_base + 1
        };
        let new_start = if new_count == 0 {
            new_base
        } else {
            new_base + 1
        };
        let hunk_hdr = format!("@@ -{old_start},{old_count} +{new_start},{new_count} @@");
        if color {
            let _ = writeln!(
                out,
                "{}",
                hunk_hdr.if_supports_color(Stream::Stdout, |t| t.cyan())
            );
        } else {
            let _ = writeln!(out, "{hunk_hdr}");
        }

        for op in &ops[start..end] {
            match op {
                DiffOp::Equal(line) => {
                    let _ = writeln!(out, " {line}");
                }
                DiffOp::Delete(line) => {
                    let text = format!("-{line}");
                    if color {
                        let _ = writeln!(
                            out,
                            "{}",
                            text.if_supports_color(Stream::Stdout, |t| t.red())
                        );
                    } else {
                        let _ = writeln!(out, "{text}");
                    }
                }
                DiffOp::Insert(line) => {
                    let text = format!("+{line}");
                    if color {
                        let _ = writeln!(
                            out,
                            "{}",
                            text.if_supports_color(Stream::Stdout, |t| t.green())
                        );
                    } else {
                        let _ = writeln!(out, "{text}");
                    }
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn compile_filter_and_display_values() {
        assert!(compile_filter(None).unwrap().is_none());
        assert!(compile_filter(Some("draft == true")).unwrap().is_some());
        assert!(compile_filter(Some("draft ==")).is_err());

        assert_eq!(format_display_value(&json!("hi")), "hi");
        assert_eq!(format_display_value(&json!(true)), "true");
        assert_eq!(format_display_value(&json!(42)), "42");
        assert_eq!(format_display_value(&json!(null)), "null");
        assert_eq!(format_display_value(&json!(["a", "b"])), r#"["a","b"]"#);
    }

    #[test]
    fn collect_markdown_files_edge_cases() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "# a\n").unwrap();
        fs::write(dir.path().join("b.markdown"), "# b\n").unwrap();
        fs::write(dir.path().join("skip.md"), "# skip\n").unwrap();
        fs::write(dir.path().join("note.txt"), "text\n").unwrap();
        fs::write(dir.path().join("cake.cook"), "---\ntitle: Cake\n---\n").unwrap();
        fs::write(dir.path().join("page.MDX"), "---\ntitle: MDX\n---\n").unwrap();

        let default_exts = crate::config::default_extensions();

        // `!`-prefixed ignore pattern works alongside plain patterns with default extensions.
        let files = collect_markdown_files(
            &[dir.path().to_path_buf()],
            &["!skip.md".to_string()],
            &default_exts,
        )
        .unwrap();
        assert_eq!(files.len(), 2);

        // Custom extensions match case-insensitively and strip optional leading dots.
        let custom_files = collect_markdown_files(
            &[dir.path().to_path_buf()],
            &[],
            &[".cook".to_string(), "mdx".to_string()],
        )
        .unwrap();
        assert_eq!(custom_files.len(), 2);
        assert!(custom_files.iter().any(|p| p.ends_with("cake.cook")));
        assert!(custom_files.iter().any(|p| p.ends_with("page.MDX")));

        // Explicit file path bypasses extension filter.
        let explicit =
            collect_markdown_files(&[dir.path().join("cake.cook")], &[], &default_exts).unwrap();
        assert_eq!(explicit, vec![dir.path().join("cake.cook")]);

        // Nonexistent path errors.
        assert!(collect_markdown_files(&[dir.path().join("nope.md")], &[], &default_exts).is_err());

        // Invalid glob pattern errors.
        assert!(collect_markdown_files(
            &[dir.path().to_path_buf()],
            &["[unclosed".to_string()],
            &default_exts
        )
        .is_err());
    }

    #[test]
    fn unified_diff_renders_changes_and_skips_identical() {
        assert!(render_unified_diff(
            "a.md",
            "---\ntitle: A\n---\n",
            "---\ntitle: A\n---\n",
            false
        )
        .is_empty());

        let diff = render_unified_diff(
            "a.md",
            "---\ntitle: Old\ndraft: true\n---\n",
            "---\ntitle: Old\ndraft: false\n---\n",
            false,
        );
        assert!(diff.contains("--- a.md\n+++ a.md\n"));
        assert!(diff.contains("@@ -1,4 +1,4 @@\n"));
        assert!(diff.contains("-draft: true\n"));
        assert!(diff.contains("+draft: false\n"));

        // Creating new frontmatter from empty prefix uses -0,0.
        let created = render_unified_diff("new.md", "", "---\ntitle: New\n---\n\n", false);
        assert!(created.contains("@@ -0,0 +1,4 @@\n"));
        assert!(created.contains("+title: New\n"));
    }
}
