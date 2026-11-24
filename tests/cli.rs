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
    let orig_toml_body = mdfm::frontmatter::split(&orig_toml).body.to_string();

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
    assert!(updated_toml.contains("# Post metadata header\n"));
    assert!(updated_toml.contains("title = \"Updated TOML\" # inline comment\n"));
    assert!(updated_toml.contains("draft = true\n"));
    assert_eq!(mdfm::frontmatter::split(&updated_toml).body, orig_toml_body);
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
    assert!(updated.starts_with("+++\n"));
    let doc = mdfm::frontmatter::split(&updated);
    assert_eq!(doc.body, orig_content);
    let fm = doc.parse_frontmatter().unwrap().unwrap();
    assert_eq!(fm.get("title"), Some(serde_json::json!("Created")));
    assert_eq!(fm.get("draft"), Some(serde_json::json!(false)));
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
}
