use anyhow::{Context, Result};
use globset::{Glob, GlobMatcher};
use owo_colors::{OwoColorize, Stream};
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::{build_table, collect_markdown_files};
use crate::config::ResolvedConfig;
use crate::frontmatter;
use crate::frontmatter::filter::{FilterContext, FilterExpr};

#[derive(Debug, Serialize)]
struct CheckEntry {
    file: String,
    valid: bool,
    errors: Vec<String>,
}

struct CompiledSchemaRule {
    matchers: Vec<GlobMatcher>,
    schema_path: PathBuf,
    base_dir: Option<PathBuf>,
}

fn compile_validator(
    schema_path: &Path,
    cache: &mut HashMap<PathBuf, jsonschema::Validator>,
) -> Result<()> {
    if cache.contains_key(schema_path) {
        return Ok(());
    }
    let schema_raw = fs::read_to_string(schema_path)
        .with_context(|| format!("Failed to read schema file '{}'", schema_path.display()))?;
    let schema_json: JsonValue = serde_json::from_str(&schema_raw)
        .with_context(|| format!("Failed to parse JSON Schema '{}'", schema_path.display()))?;
    let validator = jsonschema::validator_for(&schema_json)
        .map_err(|e| anyhow::anyhow!("Invalid JSON Schema '{}': {e}", schema_path.display()))?;
    cache.insert(schema_path.to_path_buf(), validator);
    Ok(())
}

fn normalize_for_glob(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    raw.trim_start_matches("./").to_string()
}

fn rule_matches_path(rule: &CompiledSchemaRule, file_path: &Path) -> bool {
    let candidate = if let Some(ref base) = rule.base_dir {
        if let Ok(rel) = file_path.strip_prefix(base) {
            normalize_for_glob(rel)
        } else if let (Ok(canon_file), Ok(canon_base)) =
            (fs::canonicalize(file_path), fs::canonicalize(base))
        {
            if let Ok(rel) = canon_file.strip_prefix(&canon_base) {
                normalize_for_glob(rel)
            } else {
                normalize_for_glob(file_path)
            }
        } else {
            normalize_for_glob(file_path)
        }
    } else {
        normalize_for_glob(file_path)
    };

    rule.matchers.iter().any(|m| m.is_match(&candidate))
}

pub fn execute(
    paths: &[PathBuf],
    syntax_only: bool,
    filter: Option<&FilterExpr>,
    config: &ResolvedConfig,
    out: &mut dyn Write,
) -> Result<()> {
    let effective_syntax_only =
        syntax_only || (config.schema.is_none() && config.schemas.is_empty());

    let mut validators: HashMap<PathBuf, jsonschema::Validator> = HashMap::new();
    let mut compiled_rules = Vec::new();

    if !effective_syntax_only {
        if let Some(ref default_schema) = config.schema {
            compile_validator(default_schema, &mut validators)?;
        }

        compiled_rules.reserve(config.schemas.len());
        for rule in &config.schemas {
            let mut matchers = Vec::with_capacity(rule.patterns.len());
            for pat in &rule.patterns {
                let glob = Glob::new(pat)
                    .with_context(|| format!("Invalid [[schemas]] glob pattern '{pat}'"))?;
                matchers.push(glob.compile_matcher());
            }
            compile_validator(&rule.schema, &mut validators)?;
            compiled_rules.push(CompiledSchemaRule {
                matchers,
                schema_path: rule.schema.clone(),
                base_dir: rule.base_dir.clone(),
            });
        }
    }

    let files = collect_markdown_files(paths, &config.ignore, &config.extensions)?;
    let mut results = Vec::new();

    for path in files {
        let validator = if effective_syntax_only {
            None
        } else {
            let selected_schema = compiled_rules
                .iter()
                .find(|rule| rule_matches_path(rule, &path))
                .map(|rule| rule.schema_path.as_path())
                .or(config.schema.as_deref());

            let Some(schema_path) = selected_schema else {
                continue;
            };
            Some(&validators[schema_path])
        };

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

        let (fm_format, fm_json) = match parsed {
            Some(ref fm) => match fm.to_json_value() {
                Ok(v) => (Some(fm.format()), Some(v)),
                Err(err) => {
                    results.push(CheckEntry {
                        file: path.display().to_string(),
                        valid: false,
                        errors: vec![err.to_string()],
                    });
                    continue;
                }
            },
            None => (None, None),
        };

        if let Some(expr) = filter {
            let ctx = FilterContext {
                frontmatter: fm_json.as_ref(),
                format: fm_format,
            };
            if !expr.matches_doc(ctx) {
                continue;
            }
        }

        let errors: Vec<String> = if let Some(validator) = validator {
            let instance = fm_json.unwrap_or_else(|| json!({}));
            validator
                .iter_errors(&instance)
                .map(|e| {
                    let loc = e.instance_path.to_string();
                    if loc.is_empty() {
                        e.to_string()
                    } else {
                        format!("{loc}: {e}")
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        results.push(CheckEntry {
            file: path.display().to_string(),
            valid: errors.is_empty(),
            errors,
        });
    }

    let failed_count = results.iter().filter(|r| !r.valid).count();
    let ok_count = results.len() - failed_count;
    let displayed: Vec<&CheckEntry> = if config.errors_only {
        results.iter().filter(|r| !r.valid).collect()
    } else {
        results.iter().collect()
    };

    match config.output {
        OutputFormat::Files => {
            for r in &displayed {
                writeln!(out, "{}", r.file)?;
            }
        }
        OutputFormat::Json => {
            writeln!(out, "{}", serde_json::to_string_pretty(&displayed)?)?;
        }
        OutputFormat::Plain => {
            for r in &displayed {
                if r.valid {
                    writeln!(out, "{}\tok", r.file)?;
                } else {
                    for err in &r.errors {
                        writeln!(out, "{}\terror: {err}", r.file)?;
                    }
                }
            }
            if config.errors_only {
                writeln!(out, "{ok_count} ok, {failed_count} failed")?;
            }
        }
        OutputFormat::Table => {
            if !displayed.is_empty() {
                let mut table = build_table(&["File", "Status", "Errors"]);
                for r in &displayed {
                    let status = if r.valid {
                        format!("{}", "ok".if_supports_color(Stream::Stdout, |t| t.green()))
                    } else {
                        format!("{}", "fail".if_supports_color(Stream::Stdout, |t| t.red()))
                    };
                    table.add_row(vec![r.file.clone(), status, r.errors.join("; ")]);
                }
                writeln!(out, "{table}")?;
            }
            if config.errors_only {
                writeln!(out, "{ok_count} ok, {failed_count} failed")?;
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

        // Missing schema config runs automatic syntax-only validation (valid file passes).
        let no_schema_cfg = ResolvedConfig::default();
        let mut out = Vec::new();
        execute(
            std::slice::from_ref(&valid),
            false,
            None,
            &no_schema_cfg,
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8(out).unwrap().contains("ok"));

        // Nonexistent schema file errors when schema validation is active.
        let mut out = Vec::new();
        let missing_schema_cfg = ResolvedConfig {
            schema: Some(dir.path().join("missing.json")),
            ..ResolvedConfig::default()
        };
        assert!(execute(
            std::slice::from_ref(&valid),
            false,
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
        assert!(execute(
            std::slice::from_ref(&valid),
            false,
            None,
            &bad_json_cfg,
            &mut out
        )
        .is_err());

        // Invalid JSON Schema specification errors.
        let bad_spec_schema = dir.path().join("bad_spec.json");
        fs::write(&bad_spec_schema, r#"{"type": 123}"#).unwrap();
        let bad_spec_cfg = ResolvedConfig {
            schema: Some(bad_spec_schema),
            ..ResolvedConfig::default()
        };
        assert!(execute(
            std::slice::from_ref(&valid),
            false,
            None,
            &bad_spec_cfg,
            &mut out
        )
        .is_err());

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
            false,
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
            false,
            Some(&filter),
            &json_cfg,
            &mut json_out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&json_out).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["valid"], json!(true));
    }

    #[test]
    fn check_errors_only_table_plain_and_json() {
        let dir = tempfile::tempdir().unwrap();
        let valid1 = dir.path().join("a_valid.md");
        let valid2 = dir.path().join("b_valid.md");
        let invalid = dir.path().join("c_invalid.md");
        fs::write(&valid1, "---\ntitle: One\ndraft: false\n---\nBody\n").unwrap();
        fs::write(&valid2, "---\ntitle: Two\ndraft: true\n---\nBody\n").unwrap();
        fs::write(&invalid, "---\ntitle: \"\"\ndraft: \"nope\"\n---\nBody\n").unwrap();

        let schema = dir.path().join("schema.json");
        fs::write(
            &schema,
            r#"{"type":"object","required":["title","draft"],"properties":{"title":{"type":"string","minLength":1},"draft":{"type":"boolean"}}}"#,
        )
        .unwrap();

        // Table all-pass: no table rows, only compact summary.
        let cfg = ResolvedConfig {
            output: OutputFormat::Table,
            schema: Some(schema.clone()),
            errors_only: true,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[valid1.clone(), valid2.clone()],
            false,
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "2 ok, 0 failed");

        // Table partial-fail: only failing row in table + summary line + Err.
        let mut out = Vec::new();
        assert!(execute(
            &[valid1.clone(), valid2.clone(), invalid.clone()],
            false,
            None,
            &cfg,
            &mut out
        )
        .is_err());
        let table_str = String::from_utf8(out).unwrap();
        assert!(!table_str.contains("a_valid.md"));
        assert!(!table_str.contains("b_valid.md"));
        assert!(table_str.contains("c_invalid.md"));
        assert!(table_str.trim_end().ends_with("2 ok, 1 failed"));

        // Plain all-pass: only summary line.
        let plain_cfg = ResolvedConfig {
            output: OutputFormat::Plain,
            schema: Some(schema.clone()),
            errors_only: true,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[valid1.clone(), valid2.clone()],
            false,
            None,
            &plain_cfg,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "2 ok, 0 failed");

        // Plain partial-fail: only failing errors + summary line.
        let mut out = Vec::new();
        assert!(execute(
            &[valid1.clone(), invalid.clone()],
            false,
            None,
            &plain_cfg,
            &mut out
        )
        .is_err());
        let plain_str = String::from_utf8(out).unwrap();
        assert!(!plain_str.contains("a_valid.md"));
        assert!(!plain_str.contains("\tok"));
        assert!(plain_str.contains("c_invalid.md\terror:"));
        assert!(plain_str.trim_end().ends_with("1 ok, 1 failed"));

        // JSON all-pass: empty JSON array [].
        let json_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            schema: Some(schema),
            errors_only: true,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[valid1.clone(), valid2.clone()],
            false,
            None,
            &json_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert!(parsed.is_empty());

        // JSON partial-fail: array containing only failing entries.
        let mut out = Vec::new();
        assert!(execute(&[valid1, invalid], false, None, &json_cfg, &mut out).is_err());
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["valid"], json!(false));
        assert!(parsed[0]["file"].as_str().unwrap().contains("c_invalid.md"));

        // Empty directory with errors_only: 0 ok, 0 failed / [].
        let empty_dir = tempfile::tempdir().unwrap();
        let mut out = Vec::new();
        execute(
            &[empty_dir.path().to_path_buf()],
            false,
            None,
            &cfg,
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "0 ok, 0 failed");
    }

    #[test]
    fn check_per_glob_schemas_precedence_fallback_and_errors() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("ref");
        fs::create_dir_all(&sub).unwrap();

        let ref_file = sub.join("chart.md");
        let root_file = dir.path().join("root.md");
        fs::write(&ref_file, "---\ncategory: temp\n---\nBody\n").unwrap();
        fs::write(&root_file, "---\ntitle: Root\n---\nBody\n").unwrap();

        let ref_schema = dir.path().join("ref.json");
        let default_schema = dir.path().join("default.json");
        fs::write(&ref_schema, r#"{"type":"object","required":["category"]}"#).unwrap();
        fs::write(&default_schema, r#"{"type":"object","required":["title"]}"#).unwrap();

        // First matching rule wins; unmatched file skips when no fallback schema is configured.
        let rules_only_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            schema: None,
            schemas: vec![crate::config::ResolvedSchemaRule {
                patterns: vec!["ref/**".to_string()],
                schema: ref_schema.clone(),
                base_dir: Some(dir.path().to_path_buf()),
            }],
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[dir.path().to_path_buf()],
            false,
            None,
            &rules_only_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0]["file"].as_str().unwrap().contains("chart.md"));

        // With fallback schema configured, both files validate against their respective schemas.
        let with_fallback = ResolvedConfig {
            schema: Some(default_schema),
            ..rules_only_cfg
        };
        let mut out = Vec::new();
        execute(
            &[dir.path().to_path_buf()],
            false,
            None,
            &with_fallback,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 2);

        // Invalid glob in [[schemas]] returns a clear error.
        let bad_glob_cfg = ResolvedConfig {
            schemas: vec![crate::config::ResolvedSchemaRule {
                patterns: vec!["[unclosed".to_string()],
                schema: ref_schema,
                base_dir: Some(dir.path().to_path_buf()),
            }],
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        assert!(execute(
            &[dir.path().to_path_buf()],
            false,
            None,
            &bad_glob_cfg,
            &mut out
        )
        .is_err());
    }

    #[test]
    fn check_syntax_only_yaml_toml_body_only_and_bypasses_schema() {
        let dir = tempfile::tempdir().unwrap();
        let valid_yaml = dir.path().join("valid_yaml.md");
        let valid_toml = dir.path().join("valid_toml.md");
        let body_only = dir.path().join("plain.md");
        let bad_yaml = dir.path().join("bad_yaml.md");
        let bad_toml = dir.path().join("bad_toml.md");

        fs::write(&valid_yaml, "---\ntitle: \"Ok\"\n---\nBody\n").unwrap();
        fs::write(&valid_toml, "+++\ntitle = \"Ok\"\n+++\nBody\n").unwrap();
        fs::write(&body_only, "# No frontmatter\n").unwrap();
        fs::write(&bad_yaml, "---\ntitle: :cake: Pumpkin Cake\n---\nBody\n").unwrap();
        fs::write(&bad_toml, "+++\ntitle = [unclosed\n+++\nBody\n").unwrap();

        // Automatic syntax-only mode when no schema is configured: valid YAML, TOML, body-only pass.
        let no_schema = ResolvedConfig {
            output: OutputFormat::Json,
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[valid_yaml.clone(), valid_toml.clone(), body_only.clone()],
            false,
            None,
            &no_schema,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(parsed.iter().all(|e| e["valid"] == json!(true)));

        // Malformed YAML and TOML fail in syntax-only mode.
        let mut out = Vec::new();
        assert!(execute(
            &[valid_yaml.clone(), bad_yaml.clone(), bad_toml.clone()],
            true,
            None,
            &no_schema,
            &mut out,
        )
        .is_err());
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 3);
        let by_name = |name: &str| {
            parsed
                .iter()
                .find(|e| e["file"].as_str().unwrap().ends_with(name))
                .unwrap()
        };
        assert_eq!(by_name("valid_yaml.md")["valid"], json!(true));
        assert_eq!(by_name("bad_yaml.md")["valid"], json!(false));
        assert_eq!(by_name("bad_toml.md")["valid"], json!(false));

        // Explicit --syntax-only bypasses configured schema (even strict or missing schema path).
        let strict_schema = dir.path().join("strict.json");
        fs::write(
            &strict_schema,
            r#"{"type":"object","required":["must_exist"]}"#,
        )
        .unwrap();
        let strict_cfg = ResolvedConfig {
            output: OutputFormat::Json,
            schema: Some(strict_schema),
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(
            &[valid_yaml.clone(), valid_toml, body_only],
            true,
            None,
            &strict_cfg,
            &mut out,
        )
        .unwrap();
        let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(parsed.iter().all(|e| e["valid"] == json!(true)));

        // Even a nonexistent configured schema is bypassed when syntax_only is true.
        let missing_cfg = ResolvedConfig {
            schema: Some(dir.path().join("nonexistent.json")),
            ..ResolvedConfig::default()
        };
        let mut out = Vec::new();
        execute(&[valid_yaml], true, None, &missing_cfg, &mut out).unwrap();
    }
}
