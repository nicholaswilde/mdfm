use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::cli::OutputFormat;
use crate::commands::{
    build_table, collect_markdown_files, frontmatter_prefix, render_unified_diff,
};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::{FilterContext, FilterExpr};
use crate::frontmatter::{self, Format};

/// Options controlling `mdfm fmt`.
#[derive(Debug, Clone, Default)]
pub struct FmtOptions {
    pub check: bool,
    pub sort_keys: bool,
    pub sort_arrays: Vec<String>,
    pub target_format: Option<Format>,
    pub dry_run: bool,
    pub show_diff: bool,
}

#[derive(Debug, Serialize)]
struct FmtEntry {
    file: String,
    changed: bool,
    dry_run: bool,
    check: bool,
    format: Format,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

fn status_label(entry: &FmtEntry) -> &'static str {
    if entry.changed {
        if entry.check {
            "needs formatting"
        } else if entry.dry_run {
            "would format"
        } else {
            "formatted"
        }
    } else {
        "unchanged"
    }
}

pub fn execute(
    paths: &[PathBuf],
    options: &FmtOptions,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let sort_keys = options.sort_keys || config.sort_keys;
    let sort_arrays: &[String] = if !options.sort_arrays.is_empty() {
        &options.sort_arrays
    } else {
        &config.sort_arrays
    };

    for key in sort_arrays {
        if key.is_empty() || key.split('.').any(str::is_empty) {
            anyhow::bail!("Invalid --sort-array key path: '{key}'");
        }
    }

    let yaml_opts = config.yaml_format_options();
    let files = collect_markdown_files(paths, &config.ignore, &config.extensions)?;
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

        if let Some(target) = options.target_format {
            if fm.format() != target {
                fm = fm.convert_to(target).with_context(|| {
                    format!(
                        "Failed to convert frontmatter to {target} in '{}'",
                        path.display()
                    )
                })?;
            }
        }

        for key in sort_arrays {
            fm.sort_array(key)
                .with_context(|| format!("Failed to sort array '{key}' in '{}'", path.display()))?;
        }

        if sort_keys {
            fm.sort_keys();
        }

        let rendered = doc.render_with_options(Some(&fm), yaml_opts)?;
        let changed = rendered != original;

        if changed && !options.check && !options.dry_run {
            fs::write(&path, &rendered)
                .with_context(|| format!("Failed to write '{}'", path.display()))?;
        }

        let file_str = path.display().to_string();
        let diff_text = if changed && options.show_diff {
            let before_fm = frontmatter_prefix(&original, doc.body.len());
            let after_fm = frontmatter_prefix(&rendered, doc.body.len());
            let use_color = !matches!(config.output, OutputFormat::Json);
            Some(render_unified_diff(
                &file_str, before_fm, after_fm, use_color,
            ))
        } else {
            None
        };

        results.push(FmtEntry {
            file: file_str,
            changed,
            dry_run: options.dry_run,
            check: options.check,
            format: fm.format(),
            diff: diff_text,
        });
    }

    match config.output {
        OutputFormat::Files => {
            for r in &results {
                if !options.check || r.changed {
                    writeln!(out, "{}", r.file)?;
                }
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&results)?)?;
        }
        OutputFormat::Plain | OutputFormat::Table if options.show_diff => {
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
                writeln!(out, "{}\t{}", r.file, status_label(r))?;
            }
        }
        OutputFormat::Table => {
            if !results.is_empty() {
                let mut table = build_table(&["File", "Status", "Format"]);
                for r in &results {
                    let label = status_label(r);
                    let colored = if r.changed {
                        if r.check {
                            format!("{}", label.if_supports_color(Stream::Stdout, |t| t.red()))
                        } else if r.dry_run {
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
    }

    if options.check {
        let unformatted = results.iter().filter(|r| r.changed).count();
        if unformatted > 0 {
            anyhow::bail!("Frontmatter formatting check failed for {unformatted} file(s)");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value as JsonValue};

    #[test]
    fn fmt_indents_yaml_sequences_supports_check_and_idempotency() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("post.md");
        let original = "---\ntitle: Recipe\ntags:\n- bread\n- baking\n---\n\n# Body\n";
        fs::write(&file, original).unwrap();

        let cfg = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };

        // --check fails on unindented sequences without modifying file.
        let mut check_out = Vec::new();
        assert!(execute(
            std::slice::from_ref(&file),
            &FmtOptions {
                check: true,
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut check_out,
        )
        .is_err());
        assert_eq!(fs::read_to_string(&file).unwrap(), original);
        let parsed: Vec<JsonValue> = serde_json::from_slice(&check_out).unwrap();
        assert_eq!(parsed[0]["changed"], json!(true));
        assert_eq!(parsed[0]["check"], json!(true));

        // In-place fmt indents sequences by 2 spaces while preserving body verbatim.
        let mut fmt_out = Vec::new();
        execute(
            std::slice::from_ref(&file),
            &FmtOptions::default(),
            None,
            &cfg,
            &mut fmt_out,
        )
        .unwrap();
        let expected = "---\ntitle: Recipe\ntags:\n  - bread\n  - baking\n---\n\n# Body\n";
        assert_eq!(fs::read_to_string(&file).unwrap(), expected);

        // Subsequent --check succeeds with changed == false (idempotent).
        let mut recheck_out = Vec::new();
        execute(
            std::slice::from_ref(&file),
            &FmtOptions {
                check: true,
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut recheck_out,
        )
        .unwrap();
        let parsed2: Vec<JsonValue> = serde_json::from_slice(&recheck_out).unwrap();
        assert_eq!(parsed2[0]["changed"], json!(false));

        // indent_sequences = false formats back to unindented sequences.
        let unindented_cfg = ResolvedConfig {
            indent_sequences: false,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            std::slice::from_ref(&file),
            &FmtOptions::default(),
            None,
            &unindented_cfg,
            &mut out,
        )
        .unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), original);
    }

    #[test]
    fn fmt_sort_keys_sort_array_and_format_conversion() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_file = dir.path().join("post.md");
        fs::write(
            &yaml_file,
            "---\nz_key: last\ntags:\n  - rust\n  - cli\n  - rust\n  - alpha\nauthor:\n  z: 1\n  a: 2\n---\n\nBody stays verbatim.\n",
        )
        .unwrap();

        let cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            ..ResolvedConfig::default()
        };

        // --dry-run + --diff previews sorted keys and sorted/deduped tags without writing.
        let mut diff_out = Vec::new();
        execute(
            std::slice::from_ref(&yaml_file),
            &FmtOptions {
                sort_keys: true,
                sort_arrays: vec!["tags".to_string()],
                dry_run: true,
                show_diff: true,
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut diff_out,
        )
        .unwrap();
        let diff_str = String::from_utf8(diff_out).unwrap();
        assert!(diff_str.contains("+  - alpha\n"));
        assert!(fs::read_to_string(&yaml_file)
            .unwrap()
            .starts_with("---\nz_key: last\n"));

        // Apply --sort-keys and --sort-array tags in-place.
        let mut out = Vec::new();
        execute(
            std::slice::from_ref(&yaml_file),
            &FmtOptions {
                sort_keys: true,
                sort_arrays: vec!["tags".to_string()],
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&yaml_file).unwrap(),
            "---\nauthor:\n  a: 2\n  z: 1\ntags:\n  - alpha\n  - cli\n  - rust\nz_key: last\n---\n\nBody stays verbatim.\n"
        );

        // Convert YAML -> TOML preserving body verbatim.
        let mut to_toml_out = Vec::new();
        execute(
            std::slice::from_ref(&yaml_file),
            &FmtOptions {
                target_format: Some(Format::Toml),
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut to_toml_out,
        )
        .unwrap();
        let as_toml = fs::read_to_string(&yaml_file).unwrap();
        assert!(as_toml.starts_with("+++\n"));
        assert!(as_toml.contains("tags = [\"alpha\", \"cli\", \"rust\"]\n"));
        assert!(as_toml.contains("[author]\n"));
        assert!(as_toml.ends_with("+++\n\nBody stays verbatim.\n"));

        // Convert TOML -> YAML with 4-space indent.
        let indent4_cfg = ResolvedConfig {
            yaml_indent: 4,
            ..ResolvedConfig::default()
        };
        let mut to_yaml_out = Vec::new();
        execute(
            std::slice::from_ref(&yaml_file),
            &FmtOptions {
                target_format: Some(Format::Yaml),
                sort_keys: true,
                ..FmtOptions::default()
            },
            None,
            &indent4_cfg,
            &mut to_yaml_out,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(&yaml_file).unwrap(),
            "---\nauthor:\n    a: 2\n    z: 1\ntags:\n    - alpha\n    - cli\n    - rust\nz_key: last\n---\n\nBody stays verbatim.\n"
        );
    }

    #[test]
    fn fmt_toml_sort_keys_sort_array_and_error_cases() {
        let dir = tempfile::tempdir().unwrap();
        let toml_file = dir.path().join("toml.md");
        fs::write(
            &toml_file,
            "+++\nz_title = \"Hi\"\ntags = [\"z\", \"a\", \"z\"]\n[meta]\nb = 2\na = 1\n[alpha]\nk = \"v\"\n+++\nBody\n",
        )
        .unwrap();

        let cfg = ResolvedConfig {
            output: OutputFormat::Table,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            std::slice::from_ref(&toml_file),
            &FmtOptions {
                sort_keys: true,
                sort_arrays: vec!["tags".to_string(), "missing_array".to_string()],
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        let updated = fs::read_to_string(&toml_file).unwrap();
        let tags_pos = updated.find("tags = [\"a\", \"z\"]").unwrap();
        let z_pos = updated.find("z_title = \"Hi\"").unwrap();
        let alpha_pos = updated.find("[alpha]").unwrap();
        let meta_pos = updated.find("[meta]").unwrap();
        assert!(tags_pos < z_pos);
        assert!(z_pos < alpha_pos);
        assert!(alpha_pos < meta_pos);
        // Must still parse as valid TOML!
        let reparsed = frontmatter::split(&updated)
            .parse_frontmatter()
            .unwrap()
            .unwrap();
        assert_eq!(reparsed.get("tags"), Some(json!(["a", "z"])));
        assert_eq!(reparsed.get("meta.a"), Some(json!(1)));

        // Non-array target on --sort-array returns an error.
        assert!(execute(
            std::slice::from_ref(&toml_file),
            &FmtOptions {
                sort_arrays: vec!["z_title".to_string()],
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut Vec::new(),
        )
        .is_err());

        // YAML null cannot convert to TOML -> clear error.
        let null_yaml = dir.path().join("null.md");
        fs::write(&null_yaml, "---\nsubtitle: null\n---\n").unwrap();
        assert!(execute(
            std::slice::from_ref(&null_yaml),
            &FmtOptions {
                target_format: Some(Format::Toml),
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut Vec::new(),
        )
        .is_err());

        // Body-only files are skipped silently.
        let body_only = dir.path().join("plain.md");
        fs::write(&body_only, "# Just Markdown\n").unwrap();
        let mut plain_out = Vec::new();
        execute(
            std::slice::from_ref(&body_only),
            &FmtOptions {
                check: true,
                ..FmtOptions::default()
            },
            None,
            &cfg,
            &mut plain_out,
        )
        .unwrap();
        assert!(plain_out.is_empty());
    }
}
