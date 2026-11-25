use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::{build_table, collect_markdown_files, has_matching_extension};
use crate::config::{default_extensions, ResolvedConfig};
use crate::frontmatter;
use crate::frontmatter::filter::{FilterContext, FilterExpr};

#[derive(Debug, Serialize)]
struct DelEntry {
    file: String,
    changed: bool,
    dry_run: bool,
    deleted: Vec<String>,
}

fn is_likely_path(arg: &str, extensions: &[String]) -> bool {
    let p = Path::new(arg);
    if p.exists() {
        return true;
    }
    arg == "."
        || arg == ".."
        || arg.contains('/')
        || arg.contains('\\')
        || has_matching_extension(p, extensions)
}

/// Partition positional `args` into key paths to delete (index 0 is always a key) and target paths,
/// recognizing configured file extensions when distinguishing paths from key names.
pub fn parse_del_args_with_extensions(
    args: &[String],
    extensions: &[String],
) -> Result<(Vec<String>, Vec<PathBuf>)> {
    if args.is_empty() {
        anyhow::bail!("Expected at least one key to delete");
    }

    let mut keys = Vec::new();
    let mut paths = Vec::new();
    let mut in_paths = false;

    for (idx, arg) in args.iter().enumerate() {
        if idx == 0 {
            keys.push(arg.clone());
            continue;
        }
        if !in_paths && is_likely_path(arg, extensions) {
            in_paths = true;
        }
        if in_paths {
            paths.push(PathBuf::from(arg));
        } else {
            keys.push(arg.clone());
        }
    }

    Ok((keys, paths))
}

/// Partition positional `args` into key paths to delete (index 0 is always a key) and target paths.
pub fn parse_del_args(args: &[String]) -> Result<(Vec<String>, Vec<PathBuf>)> {
    parse_del_args_with_extensions(args, &default_extensions())
}

pub fn execute(
    args: &[String],
    dry_run: bool,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let (keys, paths) = parse_del_args_with_extensions(args, &config.extensions)?;
    let files = collect_markdown_files(&paths, &config.ignore, &config.extensions)?;
    let mut results = Vec::new();

    for path in files {
        let original = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read '{}'", path.display()))?;
        let doc = frontmatter::split(&original);
        let existing_fm = doc
            .parse_frontmatter()
            .with_context(|| format!("Failed to parse frontmatter in '{}'", path.display()))?;

        let Some(mut fm) = existing_fm else {
            continue;
        };

        if let Some(expr) = filter {
            let json_obj = fm.to_json_value()?;
            let ctx = FilterContext {
                frontmatter: Some(&json_obj),
                format: Some(fm.format()),
            };
            if !expr.matches_doc(ctx) {
                continue;
            }
        }

        let mut deleted = Vec::new();
        for key in &keys {
            if fm
                .del(key)
                .with_context(|| format!("Failed to delete key '{key}' in '{}'", path.display()))?
            {
                deleted.push(key.clone());
            }
        }

        let changed = !deleted.is_empty();
        if changed && !dry_run {
            let rendered = doc.render(Some(&fm))?;
            fs::write(&path, &rendered)
                .with_context(|| format!("Failed to write '{}'", path.display()))?;
        }

        results.push(DelEntry {
            file: path.display().to_string(),
            changed,
            dry_run,
            deleted,
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
            let mut table = build_table(&["File", "Status", "Deleted Keys"]);
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
                table.add_row(vec![r.file.clone(), colored, r.deleted.join(", ")]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}

fn status_label(entry: &DelEntry) -> &'static str {
    if !entry.changed {
        "unchanged"
    } else if entry.dry_run {
        "would delete (dry-run)"
    } else {
        "deleted"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_del_args_splits_keys_and_paths() {
        let args = vec![
            "draft".to_string(),
            "author.bio".to_string(),
            "docs/post.md".to_string(),
        ];
        let (keys, paths) = parse_del_args(&args).unwrap();
        assert_eq!(keys, vec!["draft".to_string(), "author.bio".to_string()]);
        assert_eq!(paths, vec![PathBuf::from("docs/post.md")]);

        let only_keys = vec!["draft".to_string(), "old_tag".to_string()];
        let (keys2, paths2) = parse_del_args(&only_keys).unwrap();
        assert_eq!(keys2, only_keys);
        assert!(paths2.is_empty());

        assert!(parse_del_args(&[]).is_err());
    }

    #[test]
    fn del_execute_table_filters_unchanged_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let post = dir.path().join("post.md");
        let plain = dir.path().join("plain.md");
        fs::write(&post, "---\ntitle: Hello\ndraft: true\n---\nBody\n").unwrap();
        fs::write(&plain, "# Body only\n").unwrap();

        let table_cfg = ResolvedConfig {
            output: OutputFormat::Table,
            ..ResolvedConfig::default()
        };

        // Body-only file skipped -> empty table output.
        let mut out = Vec::new();
        execute(
            &["draft".to_string(), plain.to_str().unwrap().to_string()],
            false,
            None,
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(out.is_empty());

        // Non-matching --where filter skips file.
        let no_match = FilterExpr::parse("draft == false").unwrap();
        let mut out = Vec::new();
        execute(
            &[
                "draft".to_string(),
                dir.path().to_str().unwrap().to_string(),
            ],
            false,
            Some(&no_match),
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(out.is_empty());

        // Matching --where filter + missing key -> unchanged in table format.
        let yes_match = FilterExpr::parse("draft == true").unwrap();
        let mut out = Vec::new();
        execute(
            &[
                "nonexistent".to_string(),
                dir.path().to_str().unwrap().to_string(),
            ],
            false,
            Some(&yes_match),
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("unchanged"));

        // Dry-run table output.
        let mut out = Vec::new();
        execute(
            &["draft".to_string(), post.to_str().unwrap().to_string()],
            true,
            None,
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out)
            .unwrap()
            .contains("would delete (dry-run)"));

        // Actual deletion in table output.
        let mut out = Vec::new();
        execute(
            &["draft".to_string(), post.to_str().unwrap().to_string()],
            false,
            None,
            &table_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("deleted"));

        // Plain output for unchanged key.
        let plain_cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &["draft".to_string(), post.to_str().unwrap().to_string()],
            false,
            None,
            &plain_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("\tunchanged"));

        // Invalid key path and malformed frontmatter error.
        let mut err_out = Vec::new();
        assert!(execute(
            &["a..b".to_string(), post.to_str().unwrap().to_string()],
            false,
            None,
            &table_cfg,
            &mut err_out
        )
        .is_err());

        let bad = dir.path().join("bad.md");
        fs::write(&bad, "---\n[broken\n---\n").unwrap();
        assert!(execute(
            &["draft".to_string(), bad.to_str().unwrap().to_string()],
            false,
            None,
            &table_cfg,
            &mut err_out
        )
        .is_err());

        // Custom extensions recognized as path targets in parse_del_args_with_extensions.
        let (keys, paths) = parse_del_args_with_extensions(
            &["draft".to_string(), "recipe.cook".to_string()],
            &["cook".to_string()],
        )
        .unwrap();
        assert_eq!(keys, vec!["draft"]);
        assert_eq!(paths, vec![PathBuf::from("recipe.cook")]);
    }
}
