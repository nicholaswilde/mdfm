use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Preflight guard for cross-compiled QEMU environments without `binfmt_misc`.
fn can_exec_binary() -> bool {
    static CAN_EXEC: OnceLock<bool> = OnceLock::new();
    *CAN_EXEC.get_or_init(|| {
        let bin = env!("CARGO_BIN_EXE_mdfm");
        matches!(
            std::process::Command::new(bin).arg("--version").output(),
            Ok(out) if out.status.success()
        )
    })
}

fn mdfm_cmd() -> Option<Command> {
    if !can_exec_binary() {
        return None;
    }
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mdfm"));
    // Isolate from any user global config or host MDFM_* env vars.
    cmd.env_remove("MDFM_FORMAT")
        .env_remove("MDFM_OUTPUT")
        .env_remove("MDFM_SCHEMA")
        .env_remove("MDFM_CONFIG")
        .env("XDG_CONFIG_HOME", "/nonexistent-mdfm-test-xdg")
        .env("NO_COLOR", "1");
    Some(cmd)
}

fn copy_fixture(name: &str, dest_dir: &Path) -> PathBuf {
    let src = fixtures_dir().join(name);
    let dst = dest_dir.join(name);
    fs::copy(&src, &dst).expect("copy fixture");
    dst
}

#[test]
fn cli_get_yaml_and_toml_dot_paths() {
    let Some(mut cmd) = mdfm_cmd() else { return };

    // Single explicit YAML file with plain output prints raw scalar value.
    cmd.args(["--output", "plain", "get", "author.name"])
        .arg(fixtures_dir().join("yaml_post.md"))
        .assert()
        .success()
        .stdout("Nick\n");

    // Directory query in JSON returns both YAML and TOML files deterministically.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let output = cmd
        .args(["--output", "json", "get", "author.name"])
        .arg(fixtures_dir())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let entries: Vec<JsonValue> = serde_json::from_slice(&output).expect("valid JSON output");
    assert_eq!(entries.len(), 2);
    let names: Vec<&str> = entries
        .iter()
        .map(|e| e["value"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Nick"));
    assert!(names.contains(&"Ada"));

    // Table output includes headers and values.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "table", "get", "title"])
        .arg(fixtures_dir().join("toml_post.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains("TOML Post"));
}

#[test]
fn cli_where_filtering() {
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            "draft == true",
            "get",
            "title",
        ])
        .arg(fixtures_dir())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Rust Frontmatter Guide");

    // Nested key equality filter.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#"author.name == "Ada""#,
            "get",
            "title",
        ])
        .arg(fixtures_dir())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "TOML Post");

    // Array contains filter.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#"tags contains "rust""#,
            "get",
            "author.name",
        ])
        .arg(fixtures_dir())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Nick");
}

#[test]
fn cli_set_dry_run_and_body_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_file = copy_fixture("yaml_post.md", tmp.path());
    let toml_file = copy_fixture("toml_post.md", tmp.path());

    let orig_yaml = fs::read_to_string(&yaml_file).unwrap();
    let orig_yaml_body = mdfm::frontmatter::split(&orig_yaml).body.to_string();

    let orig_toml = fs::read_to_string(&toml_file).unwrap();
    let orig_toml_doc = mdfm::frontmatter::split(&orig_toml);

    // 1. Dry run must not modify the file on disk.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args([
        "--output",
        "plain",
        "set",
        "draft=false",
        "author.name=Nicholas",
        r#"tags=["rust","mdfm"]"#,
        "--dry-run",
    ])
    .arg(&yaml_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("would update (dry-run)"));

    assert_eq!(fs::read_to_string(&yaml_file).unwrap(), orig_yaml);

    // 2. Real set mutates YAML frontmatter and leaves Markdown body byte-exact untouched.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args([
        "--output",
        "json",
        "set",
        "draft=false",
        "author.name=Nicholas",
        "weight=42",
        r#"tags=["rust","mdfm"]"#,
    ])
    .arg(&yaml_file)
    .assert()
    .success();

    let updated_yaml = fs::read_to_string(&yaml_file).unwrap();
    let split_yaml = mdfm::frontmatter::split(&updated_yaml);
    assert_eq!(split_yaml.body, orig_yaml_body);

    let parsed_yaml = split_yaml.parse_frontmatter().unwrap().unwrap();
    assert_eq!(parsed_yaml.get("draft"), Some(serde_json::json!(false)));
    assert_eq!(
        parsed_yaml.get("author.name"),
        Some(serde_json::json!("Nicholas"))
    );
    assert_eq!(parsed_yaml.get("weight"), Some(serde_json::json!(42)));
    assert_eq!(
        parsed_yaml.get("tags"),
        Some(serde_json::json!(["rust", "mdfm"]))
    );

    // 3. Real set on TOML preserves comments, inline comment, and body verbatim.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["set", "title=Updated TOML", "draft=true"])
        .arg(&toml_file)
        .assert()
        .success();

    let updated_toml = fs::read_to_string(&toml_file).unwrap();
    let eol = orig_toml_doc.line_ending.as_str();
    assert!(updated_toml.contains(&format!("# Post metadata header{eol}")));
    assert!(updated_toml.contains(&format!("title = \"Updated TOML\" # inline comment{eol}")));
    assert!(updated_toml.contains(&format!("draft = true{eol}")));
    assert_eq!(
        mdfm::frontmatter::split(&updated_toml).body,
        orig_toml_doc.body
    );
}

#[test]
fn cli_set_creates_frontmatter_when_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let plain_file = copy_fixture("no_frontmatter.md", tmp.path());
    let orig_content = fs::read_to_string(&plain_file).unwrap();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--format", "toml", "set", "title=Created", "draft=false"])
        .arg(&plain_file)
        .assert()
        .success();

    let updated = fs::read_to_string(&plain_file).unwrap();
    let doc = mdfm::frontmatter::split(&updated);
    assert_eq!(
        doc.line_ending,
        mdfm::frontmatter::split(&orig_content).line_ending
    );
    assert!(updated.starts_with(&format!("+++{}", doc.line_ending.as_str())));
    assert_eq!(
        doc.body,
        format!("{}{orig_content}", doc.line_ending.as_str())
    );
    let fm = doc.parse_frontmatter().unwrap().unwrap();
    assert_eq!(fm.get("title"), Some(serde_json::json!("Created")));
    assert_eq!(fm.get("draft"), Some(serde_json::json!(false)));
}

#[test]
fn cli_set_preserves_crlf_line_endings() {
    let tmp = tempfile::tempdir().unwrap();
    let toml_file = tmp.path().join("crlf_toml.md");
    let toml_orig = "+++\r\n# Post metadata header\r\ntitle = \"TOML Post\" # inline comment\r\ndraft = false\r\n+++\r\n\r\n# Body\r\n\r\nPreserved verbatim.\r\n";
    fs::write(&toml_file, toml_orig).unwrap();
    let orig_toml_body = mdfm::frontmatter::split(toml_orig).body.to_string();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["set", "title=Updated TOML", "draft=true"])
        .arg(&toml_file)
        .assert()
        .success();

    let updated_toml = fs::read_to_string(&toml_file).unwrap();
    assert!(updated_toml.contains("# Post metadata header\r\n"));
    assert!(updated_toml.contains("title = \"Updated TOML\" # inline comment\r\n"));
    assert!(updated_toml.contains("draft = true\r\n"));
    assert_eq!(mdfm::frontmatter::split(&updated_toml).body, orig_toml_body);

    let plain_file = tmp.path().join("crlf_plain.md");
    let plain_orig = "# Plain CRLF\r\n\r\nNo frontmatter yet.\r\n";
    fs::write(&plain_file, plain_orig).unwrap();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--format", "toml", "set", "title=Created", "draft=false"])
        .arg(&plain_file)
        .assert()
        .success();

    let updated_plain = fs::read_to_string(&plain_file).unwrap();
    assert!(updated_plain.starts_with("+++\r\n"));
    assert!(updated_plain.contains("title = \"Created\"\r\n"));
    assert_eq!(
        mdfm::frontmatter::split(&updated_plain).body,
        format!("\r\n{plain_orig}")
    );
}

#[test]
fn cli_del_keys_and_dry_run() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_file = copy_fixture("yaml_post.md", tmp.path());
    let orig = fs::read_to_string(&yaml_file).unwrap();
    let orig_body = mdfm::frontmatter::split(&orig).body.to_string();

    // Dry-run does not modify disk.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args([
        "--output",
        "plain",
        "del",
        "draft",
        "author.role",
        "--dry-run",
    ])
    .arg(&yaml_file)
    .assert()
    .success()
    .stdout(predicate::str::contains("would delete (dry-run)"));
    assert_eq!(fs::read_to_string(&yaml_file).unwrap(), orig);

    // Actual deletion removes keys while keeping other keys and verbatim body.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "json", "del", "draft", "author.role"])
        .arg(&yaml_file)
        .assert()
        .success();

    let updated = fs::read_to_string(&yaml_file).unwrap();
    let doc = mdfm::frontmatter::split(&updated);
    assert_eq!(doc.body, orig_body);
    let fm = doc.parse_frontmatter().unwrap().unwrap();
    assert_eq!(fm.get("draft"), None);
    assert_eq!(fm.get("author.role"), None);
    assert_eq!(fm.get("author.name"), Some(serde_json::json!("Nick")));
    assert_eq!(
        fm.get("title"),
        Some(serde_json::json!("Rust Frontmatter Guide"))
    );
}

#[test]
fn cli_check_valid_and_invalid_schema() {
    let schema = fixtures_dir().join("schema.json");

    // Valid YAML + TOML files pass with exit code 0.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "check", "--schema"])
        .arg(&schema)
        .arg(fixtures_dir().join("yaml_post.md"))
        .arg(fixtures_dir().join("toml_post.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains("\tok"));

    // Invalid file fails with non-zero exit code and clear file-level validation errors.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "check", "--schema"])
        .arg(&schema)
        .arg(fixtures_dir().join("invalid_post.md"))
        .assert()
        .failure()
        .stdout(predicate::str::contains("invalid_post.md\terror:"))
        .stderr(predicate::str::contains("Frontmatter validation failed"));
}

#[test]
fn cli_respects_gitignore_and_local_mdfm_toml() {
    let tmp = tempfile::tempdir().unwrap();
    copy_fixture("yaml_post.md", tmp.path());
    copy_fixture("toml_post.md", tmp.path());
    copy_fixture("schema.json", tmp.path());

    // Create a gitignored file and a config-ignored file.
    fs::write(
        tmp.path().join("gitignored.md"),
        "---\ntitle: Ignored by git\ndraft: true\n---\nBody\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("config_ignored.md"),
        "---\ntitle: Ignored by config\ndraft: true\n---\nBody\n",
    )
    .unwrap();
    fs::write(tmp.path().join(".gitignore"), "gitignored.md\n").unwrap();
    fs::write(
        tmp.path().join(".mdfm.toml"),
        "default_format = \"toml\"\noutput = \"json\"\nschema = \"schema.json\"\nignore = [\"config_ignored.md\"]\n",
    )
    .unwrap();

    // Running `mdfm get title` from tmp dir picks up `.mdfm.toml` (JSON output + ignore list) and `.gitignore`.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .current_dir(tmp.path())
        .args(["get", "title"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let entries: Vec<JsonValue> = serde_json::from_slice(&out).expect("JSON from .mdfm.toml");
    assert_eq!(entries.len(), 2);
    let titles: Vec<&str> = entries
        .iter()
        .map(|e| e["value"].as_str().unwrap())
        .collect();
    assert!(titles.contains(&"Rust Frontmatter Guide"));
    assert!(titles.contains(&"TOML Post"));

    // Running `mdfm check` picks up `schema = "schema.json"` from `.mdfm.toml`.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.current_dir(tmp.path())
        .args(["check"])
        .assert()
        .success();
}

#[test]
fn in_process_run_app_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let post = copy_fixture("yaml_post.md", tmp.path());
    let schema = copy_fixture("schema.json", tmp.path());
    let cfg = tmp.path().join("explicit.toml");
    fs::write(&cfg, "output = \"plain\"\n").unwrap();

    let mut out = Vec::new();
    let cli = mdfm::Cli::try_parse_from([
        "mdfm",
        "--config",
        cfg.to_str().unwrap(),
        "get",
        "author.name",
        post.to_str().unwrap(),
    ])
    .unwrap();
    mdfm::run_app(cli, &mut out).unwrap();
    assert_eq!(String::from_utf8(out).unwrap().trim(), "Nick");

    let mut out = Vec::new();
    let cli = mdfm::Cli::try_parse_from([
        "mdfm",
        "--config",
        cfg.to_str().unwrap(),
        "set",
        "draft=false",
        post.to_str().unwrap(),
    ])
    .unwrap();
    mdfm::run_app(cli, &mut out).unwrap();

    let mut out = Vec::new();
    let cli = mdfm::Cli::try_parse_from([
        "mdfm",
        "--config",
        cfg.to_str().unwrap(),
        "check",
        "--schema",
        schema.to_str().unwrap(),
        post.to_str().unwrap(),
    ])
    .unwrap();
    mdfm::run_app(cli, &mut out).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("\tok"));

    let mut out = Vec::new();
    let cli = mdfm::Cli::try_parse_from([
        "mdfm",
        "--config",
        cfg.to_str().unwrap(),
        "del",
        "author.role",
        post.to_str().unwrap(),
    ])
    .unwrap();
    mdfm::run_app(cli, &mut out).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("\tdeleted"));
}

#[test]
fn cli_env_vars_global_config_completions_and_table_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_file = copy_fixture("yaml_post.md", tmp.path());
    let toml_file = copy_fixture("toml_post.md", tmp.path());
    let plain_file = copy_fixture("no_frontmatter.md", tmp.path());
    let schema = copy_fixture("schema.json", tmp.path());

    // Shell completions via binary.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mdfm"));

    // MDFM_FORMAT + MDFM_OUTPUT + MDFM_SCHEMA env vars.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.env("MDFM_FORMAT", "toml")
        .env("MDFM_OUTPUT", "table")
        .args(["set", "title=EnvCreated", "draft=false", "--dry-run"])
        .arg(&plain_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("would create (dry-run)"));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.env("MDFM_SCHEMA", &schema)
        .env("MDFM_OUTPUT", "table")
        .args(["--where", "draft == false", "check"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("ok"));

    // Table output for del (dry-run + deleted + unchanged) and multi-file plain get.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "get", "title"])
        .arg(&yaml_file)
        .arg(&toml_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("\tTOML Post"));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args([
        "--output",
        "table",
        "--where",
        "draft == true",
        "del",
        "draft",
        "--dry-run",
    ])
    .arg(tmp.path())
    .assert()
    .success()
    .stdout(predicate::str::contains("would delete (dry-run)"));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "table", "del", "missing_key"])
        .arg(&toml_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("unchanged"));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "table", "del", "draft"])
        .arg(&yaml_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));

    // Global config via XDG_CONFIG_HOME (YAML) and explicit MDFM_CONFIG (JSON).
    let xdg = tmp.path().join("xdg");
    let mdfm_dir = xdg.join("mdfm");
    fs::create_dir_all(&mdfm_dir).unwrap();
    fs::write(
        mdfm_dir.join("config.yaml"),
        "default_format: yaml\noutput: json\n",
    )
    .unwrap();
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.env("XDG_CONFIG_HOME", &xdg)
        .args(["get", "title"])
        .arg(&toml_file)
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""value": "TOML Post""#));

    let json_cfg = tmp.path().join("cfg.json");
    fs::write(&json_cfg, r#"{"output":"plain"}"#).unwrap();
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.env("MDFM_CONFIG", &json_cfg)
        .args(["get", "title"])
        .arg(&toml_file)
        .assert()
        .success()
        .stdout("TOML Post\n");

    // Error paths: invalid --where, missing schema, nonexistent path.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--where", "draft ==", "get", "title"])
        .arg(&toml_file)
        .assert()
        .failure();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["check"])
        .arg(&toml_file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No JSON Schema specified"));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["get", "title"])
        .arg(tmp.path().join("does-not-exist.md"))
        .assert()
        .failure();

    // Update command: help & non-interactive confirmation guard (no network call).
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--version"));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["update"])
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot confirm update in non-interactive mode",
        ));

    // Man page generation via binary.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["man"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".TH mdfm"))
        .stdout(predicate::str::contains(".SH SYNOPSIS"));
}

#[test]
fn cli_del_preserves_yaml_key_order() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("recipe.md");
    let orig = "\
---
source: Recipe Box
servings: 1
prep time: 5 minutes
cook time: 0 minutes
total time: 5 minutes
hero: assets/images/fireside-special.webp
---

# Recipe
";
    fs::write(&file, orig).unwrap();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["del", "prep time"]).arg(&file).assert().success();

    let updated = fs::read_to_string(&file).unwrap();
    let expected = "\
---
source: Recipe Box
servings: 1
cook time: 0 minutes
total time: 5 minutes
hero: assets/images/fireside-special.webp
---

# Recipe
";
    assert_eq!(updated, expected);
}

#[test]
fn cli_set_new_frontmatter_blank_line_spacing() {
    let tmp = tempfile::tempdir().unwrap();

    let heading_file = tmp.path().join("nofm.md");
    fs::write(&heading_file, "# Heading\n\nBody text.\n").unwrap();

    let spaced_file = tmp.path().join("spaced.md");
    fs::write(&spaced_file, "\n# Already Spaced\n").unwrap();

    let empty_file = tmp.path().join("empty.md");
    fs::write(&empty_file, "").unwrap();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["set", "comments=true"])
        .arg(tmp.path())
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&heading_file).unwrap(),
        "---\ncomments: true\n---\n\n# Heading\n\nBody text.\n"
    );
    assert_eq!(
        fs::read_to_string(&spaced_file).unwrap(),
        "---\ncomments: true\n---\n\n# Already Spaced\n"
    );
    assert_eq!(
        fs::read_to_string(&empty_file).unwrap(),
        "---\ncomments: true\n---\n"
    );
}

#[test]
fn cli_set_skips_write_when_semantically_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("indented.md");
    let orig = "\
---
title: Savory Star Bread
comments: true

tags:
  - bread
  - must-try
---

# Body
";
    fs::write(&file, orig).unwrap();

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "set", "comments=true"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("\tunchanged"));

    assert_eq!(fs::read_to_string(&file).unwrap(), orig);
}

#[test]
fn cli_set_array_append_and_remove_yaml_and_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_file = tmp.path().join("bread.md");
    let yaml_orig = "\
---
title: Savory Star Bread
tags:
  - bread
  - applied-kitchen
  - draft
---

# Savory Star Bread
";
    fs::write(&yaml_file, yaml_orig).unwrap();

    let toml_file = tmp.path().join("guide.md");
    let toml_orig = "\
+++
title = \"CLI Guide\"
tags = [\"rust\", \"draft\"] # inline comment
+++

# CLI Guide
";
    fs::write(&toml_file, toml_orig).unwrap();

    let plain_file = tmp.path().join("notes.md");
    fs::write(&plain_file, "# Plain Notes\n").unwrap();

    // Append duplicate tag is a no-op preserving original 2-space YAML formatting.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "set", "tags+=bread"])
        .arg(&yaml_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("\tunchanged"));
    assert_eq!(fs::read_to_string(&yaml_file).unwrap(), yaml_orig);

    // Filtered append: --where 'tags contains "applied-kitchen"' set tags+=featured
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args([
        "--where",
        "tags contains \"applied-kitchen\"",
        "set",
        "tags+=featured",
        r#"tags+=["must-try","featured"]"#,
    ])
    .arg(tmp.path())
    .assert()
    .success();

    let yaml_doc_str = fs::read_to_string(&yaml_file).unwrap();
    let yaml_fm = mdfm::frontmatter::split(&yaml_doc_str)
        .parse_frontmatter()
        .unwrap()
        .unwrap();
    assert_eq!(
        yaml_fm.get("tags"),
        Some(serde_json::json!([
            "bread",
            "applied-kitchen",
            "draft",
            "featured",
            "must-try"
        ]))
    );
    // Unmatched TOML and body-only files were untouched.
    assert_eq!(fs::read_to_string(&toml_file).unwrap(), toml_orig);
    assert_eq!(fs::read_to_string(&plain_file).unwrap(), "# Plain Notes\n");

    // Bulk remove across directory removes "draft" from YAML + TOML and leaves plain file body-only.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["set", "tags-=draft", "tags+=v1"])
        .arg(&toml_file)
        .assert()
        .success();

    let toml_updated = fs::read_to_string(&toml_file).unwrap();
    assert!(toml_updated.contains("# inline comment"));
    let toml_fm = mdfm::frontmatter::split(&toml_updated)
        .parse_frontmatter()
        .unwrap()
        .unwrap();
    assert_eq!(toml_fm.get("tags"), Some(serde_json::json!(["rust", "v1"])));

    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["set", "tags-=draft"])
        .arg(tmp.path())
        .assert()
        .success();
    assert_eq!(fs::read_to_string(&plain_file).unwrap(), "# Plain Notes\n");
    let yaml_after_rm = fs::read_to_string(&yaml_file).unwrap();
    let yaml_fm_after = mdfm::frontmatter::split(&yaml_after_rm)
        .parse_frontmatter()
        .unwrap()
        .unwrap();
    assert_eq!(
        yaml_fm_after.get("tags"),
        Some(serde_json::json!([
            "bread",
            "applied-kitchen",
            "featured",
            "must-try"
        ]))
    );

    // Non-array key fails loudly.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["set", "title+=nope"])
        .arg(&yaml_file)
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not an array"));
}

#[test]
fn cli_where_keys_with_spaces_and_brackets() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_recipe = tmp.path().join("bread.md");
    let toml_recipe = tmp.path().join("toast.md");
    let null_recipe = tmp.path().join("note.md");

    fs::write(
        &yaml_recipe,
        "---\ntitle: Savory Star Bread\nprep time: \"10 minutes\"\ncook time: \"25 minutes\"\ntags:\n  - bread\n  - baking\nmeta:\n  prep time: \"5 minutes\"\n  v1.2: true\n---\n\n# Bread\n",
    )
    .unwrap();
    fs::write(
        &toml_recipe,
        "+++\ntitle = \"Quick Toast\"\n\"prep time\" = \"2 minutes\"\ntags = [\"fast\"]\n\n[meta]\n\"prep time\" = \"2 minutes\"\n+++\n\n# Toast\n",
    )
    .unwrap();
    fs::write(
        &null_recipe,
        "---\ntitle: Plain Note\nprep time: null\n---\n\n# Note\n",
    )
    .unwrap();

    // Quoted LHS key with spaces (`"prep time" != null`).
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#""prep time" != null"#,
            "get",
            "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 2);

    // Backtick-quoted key with spaces.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#"`prep time` == "10 minutes""#,
            "get",
            "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Savory Star Bread");

    // Dotted + bracketed key with spaces on TOML file.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#"meta["prep time"] == "2 minutes""#,
            "get",
            "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Quick Toast");

    // Root bracket key + literal dot key inside brackets.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#"["cook time"] == "25 minutes" and meta["v1.2"] == true"#,
            "get",
            "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Savory Star Bread");

    // Bracket array index.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output",
            "json",
            "--where",
            r#"tags[0] == "fast""#,
            "get",
            "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Quick Toast");

    // Unclosed bracket errors clearly.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--where", r#"meta["prep time""#, "get", "title"])
        .arg(&yaml_recipe)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid --where expression"));
}

#[test]
fn cli_configurable_extensions_flag_and_config() {
    let tmp = tempfile::tempdir().unwrap();
    let md_file = tmp.path().join("post.md");
    let cook_file = tmp.path().join("pumpkin_cake.cook");
    let mdx_file = tmp.path().join("guide.MDX");
    let txt_file = tmp.path().join("notes.txt");

    fs::write(
        &md_file,
        "---\ntitle: Markdown Post\ndraft: true\n---\nBody\n",
    )
    .unwrap();
    fs::write(
        &cook_file,
        "---\ntitle: Pumpkin Cake\ntags:\n  - dessert\n---\nMix @flour{2%cups}.\n",
    )
    .unwrap();
    fs::write(&mdx_file, "---\ntitle: MDX Guide\n---\n<Alert />\n").unwrap();
    fs::write(&txt_file, "---\ntitle: Ignored Txt\n---\nPlain\n").unwrap();

    // Default walk discovers only .md / .markdown.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--output", "json", "get", "title"])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["value"], "Markdown Post");

    // Repeatable --ext with leading dot and mixed case discovers .cook and .MDX.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--output", "json", "--ext", ".cook", "--ext", "mdx", "get", "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 2);
    let titles: Vec<&str> = entries
        .iter()
        .map(|e| e["value"].as_str().unwrap())
        .collect();
    assert!(titles.contains(&"Pumpkin Cake"));
    assert!(titles.contains(&"MDX Guide"));

    // `.mdfm.toml` extensions list applies to walks, and CLI `--ext` overrides it.
    let cfg = tmp.path().join(".mdfm.toml");
    fs::write(&cfg, "extensions = [\"cook\", \"md\"]\n").unwrap();

    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .current_dir(tmp.path())
        .args(["--output", "json", "get", "title", "."])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 2);

    // Mutate .cook file via directory walk using config extensions.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.current_dir(tmp.path())
        .args(["--ext", "cook", "set", "tags+=autumn", "."])
        .assert()
        .success();
    let updated_cook = fs::read_to_string(&cook_file).unwrap();
    assert!(updated_cook.contains("autumn"));
    assert!(updated_cook.ends_with("Mix @flour{2%cups}.\n"));

    // Invalid empty --ext fails cleanly.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--ext", ".", "get", "title"])
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid empty file extension"));
}

#[test]
fn cli_check_errors_only_quiet_and_config() {
    let tmp = tempfile::tempdir().unwrap();
    let valid_a = tmp.path().join("a_valid.md");
    let valid_b = tmp.path().join("b_valid.md");
    let invalid = tmp.path().join("c_invalid.md");
    let schema = copy_fixture("schema.json", tmp.path());

    fs::write(&valid_a, "---\ntitle: Alpha\ndraft: false\n---\nBody\n").unwrap();
    fs::write(&valid_b, "---\ntitle: Beta\ndraft: true\n---\nBody\n").unwrap();
    fs::write(&invalid, "---\ntitle: \"\"\ndraft: \"wrong\"\n---\nBody\n").unwrap();

    // All-pass with --errors-only in table format prints only summary line.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["check", "--errors-only", "--schema"])
        .arg(&schema)
        .arg(&valid_a)
        .arg(&valid_b)
        .assert()
        .success()
        .stdout(predicate::str::diff("2 ok, 0 failed\n"));

    // Partial-fail with -q in plain format prints only failing file errors + summary.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "check", "-q", "--schema"])
        .arg(&schema)
        .arg(&valid_a)
        .arg(&invalid)
        .assert()
        .failure()
        .stdout(predicate::str::contains("a_valid.md").not())
        .stdout(predicate::str::contains("c_invalid.md\terror:"))
        .stdout(predicate::str::contains("1 ok, 1 failed"));

    // All-pass with --quiet in JSON format emits empty JSON array [].
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--output", "json", "check", "--quiet", "--schema"])
        .arg(&schema)
        .arg(&valid_a)
        .arg(&valid_b)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert!(parsed.is_empty());

    // Partial-fail with --errors-only in JSON format emits only failing entries.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--output", "json", "check", "--errors-only", "--schema"])
        .arg(&schema)
        .arg(&valid_a)
        .arg(&invalid)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let parsed: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["valid"], false);

    // Config toggle `errors_only = true` in .mdfm.toml works without CLI flag.
    fs::write(
        tmp.path().join(".mdfm.toml"),
        format!(
            "schema = \"{}\"\nerrors_only = true\n",
            schema.file_name().unwrap().to_str().unwrap()
        ),
    )
    .unwrap();
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.current_dir(tmp.path())
        .args(["check", "a_valid.md", "b_valid.md"])
        .assert()
        .success()
        .stdout(predicate::str::diff("2 ok, 0 failed\n"));
}

#[test]
fn cli_get_multi_key_and_root_dump() {
    let tmp = tempfile::tempdir().unwrap();
    let a = tmp.path().join("a.md");
    let b = tmp.path().join("b.md");
    let c = tmp.path().join("c_plain.md");

    fs::write(
        &a,
        "---\ntitle: Alpha\ntags:\n  - rust\n  - cli\nauthor:\n  name: Nick\n---\nBody\n",
    )
    .unwrap();
    fs::write(&b, "+++\ntitle = \"Beta\"\n+++\nBody\n").unwrap();
    fs::write(&c, "# No Frontmatter\n").unwrap();

    // Comma-separated multi-key query in JSON mode (missing keys -> null).
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--output", "json", "get", "title,tags,author.name"])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["values"]["title"], "Alpha");
    assert_eq!(
        entries[0]["values"]["tags"],
        serde_json::json!(["rust", "cli"])
    );
    assert_eq!(entries[0]["values"]["author.name"], "Nick");
    assert_eq!(entries[1]["values"]["title"], "Beta");
    assert_eq!(entries[1]["values"]["tags"], JsonValue::Null);
    assert_eq!(entries[1]["values"]["author.name"], JsonValue::Null);

    // Repeatable -k / --key in table mode renders multi-column table.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["get", "-k", "title", "-k", "tags"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("tags"))
        .stdout(predicate::str::contains("Alpha"))
        .stdout(predicate::str::contains("Beta"));

    // Multi-key plain output on directory is tab-delimited `<file>\t<v1>\t<v2>`.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["--output", "plain", "get", "title,author.name"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\tAlpha\tNick\n"))
        .stdout(predicate::str::contains("\tBeta\t\n"));

    // Root selector `.` dumps complete frontmatter JSON per file.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--output", "json", "get", "."])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entries: Vec<JsonValue> = serde_json::from_slice(&out).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["key"], ".");
    assert_eq!(entries[0]["value"]["title"], "Alpha");
    assert_eq!(entries[0]["value"]["author"]["name"], "Nick");
    assert_eq!(entries[1]["value"]["title"], "Beta");

    // Single explicit file with `.` in plain mode prints raw JSON object (jq-pipeable).
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--output", "plain", "get", "."])
        .arg(&b)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let obj: JsonValue = serde_json::from_slice(&out).unwrap();
    assert_eq!(obj["title"], "Beta");

    // Mixing `.` with another key errors cleanly.
    let Some(mut cmd) = mdfm_cmd() else { return };
    cmd.args(["get", ".,title"])
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Cannot combine root selector"));
}

#[test]
fn cli_files_only_and_has_frontmatter_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let yaml_file = tmp.path().join("a_yaml.md");
    let toml_file = tmp.path().join("b_toml.md");
    let empty_fm_file = tmp.path().join("c_empty_fm.md");
    let body_only_file = tmp.path().join("d_body_only.md");
    let schema = copy_fixture("schema.json", tmp.path());

    fs::write(&yaml_file, "---\ntitle: YAML\ndraft: false\n---\nBody\n").unwrap();
    fs::write(
        &toml_file,
        "+++\ntitle = \"TOML\"\ndraft = true\n+++\nBody\n",
    )
    .unwrap();
    fs::write(&empty_fm_file, "---\n---\nEmpty block\n").unwrap();
    fs::write(&body_only_file, "# Just Markdown\n").unwrap();

    // `-l` with `not has_frontmatter` finds ONLY body-only file (distinguishes from empty block).
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--where", "not has_frontmatter", "-l", "get", "."])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("d_body_only.md"));

    // `--output files` with `frontmatter_format == "toml"` lists only TOML file.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--where",
            r#"frontmatter_format == "toml""#,
            "--output",
            "files",
            "get",
            "title",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("b_toml.md"));

    // `set` with `--where 'has_frontmatter == false'` and `-l` creates frontmatter ONLY on body-only file.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args([
            "--where",
            "has_frontmatter == false",
            "-l",
            "set",
            "comments=true",
        ])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("d_body_only.md"));
    assert!(fs::read_to_string(&body_only_file)
        .unwrap()
        .contains("comments: true"));
    assert!(!fs::read_to_string(&empty_fm_file)
        .unwrap()
        .contains("comments"));

    // `del` with `--files-only` prints matching file paths one per line.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["--files-only", "--where", "draft == true", "del", "draft"])
        .arg(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("b_toml.md"));

    // `check -q -l` prints ONLY failing file paths (no summary line) and exits non-zero.
    let Some(mut cmd) = mdfm_cmd() else { return };
    let out = cmd
        .args(["-l", "check", "-q", "--schema"])
        .arg(&schema)
        .arg(&yaml_file)
        .arg(&empty_fm_file)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("c_empty_fm.md"));
}
