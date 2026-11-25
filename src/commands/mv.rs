use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::{
    build_table, collect_markdown_files, frontmatter_prefix, render_unified_diff,
};
use crate::config::ResolvedConfig;
use crate::frontmatter::filter::{FilterContext, FilterExpr};
use crate::frontmatter::{self, FrontmatterData};

/// Options controlling `mdfm mv` / `mdfm rename`.
#[derive(Debug, Clone, Copy, Default)]
pub struct MvOptions {
    pub merge: bool,
    pub overwrite: bool,
    pub dry_run: bool,
    pub show_diff: bool,
}

#[derive(Debug, Serialize)]
struct MvEntry {
    file: String,
    old_key: String,
    new_key: String,
    changed: bool,
    merged: bool,
    dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

fn validate_key_path(key: &str) -> Result<()> {
    if key.is_empty() || key.split('.').any(str::is_empty) {
        anyhow::bail!("Invalid key path: '{key}'");
    }
    Ok(())
}

fn is_descendant_path(child: &str, parent: &str) -> bool {
    child
        .strip_prefix(parent)
        .is_some_and(|rest| rest.starts_with('.'))
}

fn set_and_delete(
    fm: &mut FrontmatterData,
    old_key: &str,
    new_key: &str,
    value: JsonValue,
    path: &Path,
) -> Result<()> {
    if is_descendant_path(new_key, old_key) {
        fm.del(old_key)
            .with_context(|| format!("Failed to delete key '{old_key}' in '{}'", path.display()))?;
        fm.set(new_key, value)
            .with_context(|| format!("Failed to set key '{new_key}' in '{}'", path.display()))?;
    } else {
        fm.set(new_key, value)
            .with_context(|| format!("Failed to set key '{new_key}' in '{}'", path.display()))?;
        fm.del(old_key)
            .with_context(|| format!("Failed to delete key '{old_key}' in '{}'", path.display()))?;
    }
    Ok(())
}

fn merge_into_destination(
    fm: &mut FrontmatterData,
    old_key: &str,
    new_key: &str,
    old_val: JsonValue,
    dest_val: JsonValue,
    path: &Path,
) -> Result<()> {
    match (old_val, dest_val) {
        (
            old @ (JsonValue::Array(_)
            | JsonValue::String(_)
            | JsonValue::Number(_)
            | JsonValue::Bool(_)),
            JsonValue::Array(_),
        ) => {
            fm.append(new_key, old).with_context(|| {
                format!(
                    "Failed to merge key '{old_key}' into array '{new_key}' in '{}'",
                    path.display()
                )
            })?;
        }
        (
            JsonValue::Array(old_items),
            dest @ (JsonValue::String(_) | JsonValue::Number(_) | JsonValue::Bool(_)),
        ) => {
            let mut combined = vec![dest];
            for item in old_items {
                if !combined.iter().any(|e| e == &item) {
                    combined.push(item);
                }
            }
            fm.set(new_key, JsonValue::Array(combined))
                .with_context(|| {
                    format!(
                        "Failed to merge array '{old_key}' into '{new_key}' in '{}'",
                        path.display()
                    )
                })?;
        }
        (JsonValue::Object(old_map), JsonValue::Object(mut dest_map)) => {
            let mut needs_full_set = false;
            let mut missing_entries: Vec<(String, JsonValue)> = Vec::new();
            for (k, v) in old_map {
                if !dest_map.contains_key(&k) {
                    if k.contains('.') {
                        needs_full_set = true;
                    }
                    dest_map.insert(k.clone(), v.clone());
                    missing_entries.push((k, v));
                }
            }
            if needs_full_set {
                fm.set(new_key, JsonValue::Object(JsonMap::from_iter(dest_map)))
                    .with_context(|| {
                        format!(
                            "Failed to merge mapping '{old_key}' into '{new_key}' in '{}'",
                            path.display()
                        )
                    })?;
            } else {
                for (k, v) in missing_entries {
                    let sub_path = format!("{new_key}.{k}");
                    fm.set(&sub_path, v).with_context(|| {
                        format!(
                            "Failed to merge key '{old_key}.{k}' into '{sub_path}' in '{}'",
                            path.display()
                        )
                    })?;
                }
            }
        }
        (old, dest) if old == dest => {}
        _ => {
            anyhow::bail!(
                "Cannot merge key '{old_key}' into '{new_key}' in '{}': incompatible or conflicting values (use --overwrite to replace)",
                path.display()
            );
        }
    }
    Ok(())
}

fn apply_move(
    fm: &mut FrontmatterData,
    old_key: &str,
    new_key: &str,
    options: MvOptions,
    path: &Path,
) -> Result<(bool, bool)> {
    if old_key == new_key {
        return Ok((false, false));
    }

    let Some(old_val) = fm.get(old_key) else {
        return Ok((false, false));
    };

    match fm.get(new_key) {
        None => {
            set_and_delete(fm, old_key, new_key, old_val, path)?;
            Ok((true, false))
        }
        Some(_) if options.overwrite => {
            set_and_delete(fm, old_key, new_key, old_val, path)?;
            Ok((true, false))
        }
        Some(dest_val) if options.merge => {
            merge_into_destination(fm, old_key, new_key, old_val, dest_val, path)?;
            fm.del(old_key).with_context(|| {
                format!("Failed to delete key '{old_key}' in '{}'", path.display())
            })?;
            Ok((true, true))
        }
        Some(_) => {
            anyhow::bail!(
                "Destination key '{new_key}' already exists in '{}' (use --overwrite or --merge)",
                path.display()
            );
        }
    }
}

pub fn execute(
    old_key: &str,
    new_key: &str,
    paths: &[PathBuf],
    options: MvOptions,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    validate_key_path(old_key)?;
    validate_key_path(new_key)?;

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

        let (mutated, merged) = apply_move(&mut fm, old_key, new_key, options, &path)?;

        let file_str = path.display().to_string();
        let (changed, diff_text) = if mutated {
            let rendered = doc.render(Some(&fm))?;
            let bytes_changed = rendered != original;
            if bytes_changed && !options.dry_run {
                fs::write(&path, &rendered)
                    .with_context(|| format!("Failed to write '{}'", path.display()))?;
            }
            let diff = if bytes_changed && options.show_diff {
                let before_fm = frontmatter_prefix(&original, doc.body.len());
                let after_fm = frontmatter_prefix(&rendered, doc.body.len());
                let use_color = !matches!(config.output, OutputFormat::Json);
                Some(render_unified_diff(
                    &file_str, before_fm, after_fm, use_color,
                ))
            } else {
                None
            };
            (bytes_changed, diff)
        } else {
            (false, None)
        };

        results.push(MvEntry {
            file: file_str,
            old_key: old_key.to_string(),
            new_key: new_key.to_string(),
            changed,
            merged,
            dry_run: options.dry_run,
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
                let status = status_label(r);
                writeln!(out, "{}\t{status}", r.file)?;
            }
        }
        OutputFormat::Table => {
            if results.is_empty() {
                return Ok(());
            }
            let mut table = build_table(&["File", "Status", "Mapping"]);
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
                table.add_row(vec![
                    r.file.clone(),
                    colored,
                    format!("{} -> {}", r.old_key, r.new_key),
                ]);
            }
            writeln!(out, "{table}")?;
        }
    }

    Ok(())
}

fn status_label(entry: &MvEntry) -> &'static str {
    if !entry.changed {
        "unchanged"
    } else if entry.dry_run {
        if entry.merged {
            "would merge (dry-run)"
        } else {
            "would rename (dry-run)"
        }
    } else if entry.merged {
        "merged"
    } else {
        "renamed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mv_rename_merge_conflict_and_nested_yaml_toml() {
        let dir = tempfile::tempdir().unwrap();
        let yaml_file = dir.path().join("recipe.md");
        let toml_file = dir.path().join("post.md");
        fs::write(
            &yaml_file,
            "---\ntitle: Puree\ntag:\n  - gordon-ramsay\n  - puree\n  - side\ntags:\n  - sides\n  - puree\n---\nBody\n",
        )
        .unwrap();
        fs::write(
            &toml_file,
            "+++\ntitle = \"Post\"\n\n[author]\nfull_name = \"Nick\"\n+++\nBody\n",
        )
        .unwrap();

        let cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            ..ResolvedConfig::default()
        };

        // Conflict errors without --overwrite or --merge.
        let mut out = Vec::new();
        assert!(execute(
            "tag",
            "tags",
            std::slice::from_ref(&yaml_file),
            MvOptions::default(),
            None,
            &cfg,
            &mut out
        )
        .is_err());

        // --merge deduplicates array union and removes source key.
        let mut out = Vec::new();
        execute(
            "tag",
            "tags",
            std::slice::from_ref(&yaml_file),
            MvOptions {
                merge: true,
                ..MvOptions::default()
            },
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("\tmerged"));
        let updated = fs::read_to_string(&yaml_file).unwrap();
        let doc = frontmatter::split(&updated);
        let fm = doc.parse_frontmatter().unwrap().unwrap();
        assert!(fm.get("tag").is_none());
        assert_eq!(
            fm.get("tags"),
            Some(json!(["sides", "puree", "gordon-ramsay", "side"]))
        );

        // Nested rename on TOML preserves body and moves author.full_name -> author.name.
        let mut out = Vec::new();
        execute(
            "author.full_name",
            "author.name",
            std::slice::from_ref(&toml_file),
            MvOptions::default(),
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("\trenamed"));
        let updated_toml = fs::read_to_string(&toml_file).unwrap();
        let doc = frontmatter::split(&updated_toml);
        let fm = doc.parse_frontmatter().unwrap().unwrap();
        assert!(fm.get("author.full_name").is_none());
        assert_eq!(fm.get("author.name"), Some(json!("Nick")));
        assert_eq!(doc.body, "Body\n");

        // Shallow-merge mappings with --merge (destination keys win on conflict).
        let map_file = dir.path().join("map.md");
        fs::write(
            &map_file,
            "---\nold_meta:\n  a: from_old\n  b: extra\nmeta:\n  a: from_dest\n---\nBody\n",
        )
        .unwrap();
        let mut out = Vec::new();
        execute(
            "old_meta",
            "meta",
            std::slice::from_ref(&map_file),
            MvOptions {
                merge: true,
                ..MvOptions::default()
            },
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        let map_updated = fs::read_to_string(&map_file).unwrap();
        let fm = frontmatter::split(&map_updated)
            .parse_frontmatter()
            .unwrap()
            .unwrap();
        assert!(fm.get("old_meta").is_none());
        assert_eq!(fm.get("meta.a"), Some(json!("from_dest")));
        assert_eq!(fm.get("meta.b"), Some(json!("extra")));

        // Incompatible scalar conflict fails --merge, succeeds with --overwrite.
        let conflict = dir.path().join("conflict.md");
        fs::write(&conflict, "---\nold: A\nnew: B\n---\nBody\n").unwrap();
        let mut out = Vec::new();
        assert!(execute(
            "old",
            "new",
            std::slice::from_ref(&conflict),
            MvOptions {
                merge: true,
                ..MvOptions::default()
            },
            None,
            &cfg,
            &mut out
        )
        .is_err());

        let mut out = Vec::new();
        execute(
            "old",
            "new",
            std::slice::from_ref(&conflict),
            MvOptions {
                overwrite: true,
                ..MvOptions::default()
            },
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        let fm = frontmatter::split(&fs::read_to_string(&conflict).unwrap())
            .parse_frontmatter()
            .unwrap()
            .unwrap();
        assert!(fm.get("old").is_none());
        assert_eq!(fm.get("new"), Some(json!("A")));

        // Invalid key path errors upfront.
        let mut out = Vec::new();
        assert!(execute(
            "a..b",
            "c",
            std::slice::from_ref(&yaml_file),
            MvOptions::default(),
            None,
            &cfg,
            &mut out
        )
        .is_err());
    }
}
