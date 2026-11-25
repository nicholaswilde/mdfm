use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::frontmatter::Format;

/// Supported configuration file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

impl ConfigFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "toml" => Some(Self::Toml),
            "yaml" | "yml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Default file extensions discovered during directory walks.
pub const DEFAULT_EXTENSIONS: &[&str] = &["md", "markdown"];

/// Return the default list of discovered file extensions (`["md", "markdown"]`).
pub fn default_extensions() -> Vec<String> {
    DEFAULT_EXTENSIONS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Normalize configured file extensions: trim whitespace, strip leading `.`, lowercase,
/// deduplicate while preserving order, and reject empty entries or empty lists.
pub fn normalize_extensions(raw: &[String]) -> Result<Vec<String>> {
    let mut normalized = Vec::with_capacity(raw.len());
    for entry in raw {
        let cleaned = entry.trim().trim_start_matches('.').to_ascii_lowercase();
        if cleaned.is_empty() {
            anyhow::bail!("Invalid empty file extension '{entry}'");
        }
        if !normalized.contains(&cleaned) {
            normalized.push(cleaned);
        }
    }
    if normalized.is_empty() {
        anyhow::bail!("Configured file extensions list cannot be empty");
    }
    Ok(normalized)
}

/// Representation of values loaded from a configuration file (`.mdfm.toml` or global config).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFile {
    pub default_format: Option<Format>,
    pub output: Option<OutputFormat>,
    pub schema: Option<PathBuf>,
    pub ignore: Option<Vec<String>>,
    pub extensions: Option<Vec<String>>,
    pub errors_only: Option<bool>,
}

impl ConfigFile {
    /// Parse configuration file content in the specified format.
    pub fn parse(content: &str, format: ConfigFormat) -> Result<Self> {
        match format {
            ConfigFormat::Toml => {
                toml::from_str(content).context("Failed to parse TOML configuration")
            }
            ConfigFormat::Yaml => {
                serde_yaml_ng::from_str(content).context("Failed to parse YAML configuration")
            }
            ConfigFormat::Json => {
                serde_json::from_str(content).context("Failed to parse JSON configuration")
            }
        }
    }

    /// Load and parse a configuration file from disk.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration file '{}'", path.display()))?;

        let format = ConfigFormat::from_path(path).ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported configuration format for '{}'. Expected .toml, .yaml, .yml, or .json",
                path.display()
            )
        })?;

        Self::parse(&content, format)
    }

    /// Discover and load the user-level global configuration file if present.
    pub fn load_global() -> Result<Option<(PathBuf, Self)>> {
        for path in default_global_config_paths() {
            if path.is_file() {
                let cfg = Self::load_from_path(&path)?;
                return Ok(Some((path, cfg)));
            }
        }
        Ok(None)
    }
}

/// Standard candidate locations for user-global configuration files.
pub fn default_global_config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|xdg| !xdg.is_empty())
        .map(|xdg| PathBuf::from(xdg).join("mdfm"))
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .filter(|h| !h.is_empty())
                .map(|h| PathBuf::from(h).join(".config").join("mdfm"))
        })
        .or_else(|| {
            std::env::var("APPDATA")
                .ok()
                .filter(|a| !a.is_empty())
                .map(|a| PathBuf::from(a).join("mdfm"))
        })
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .filter(|u| !u.is_empty())
                .map(|u| PathBuf::from(u).join(".config").join("mdfm"))
        });

    if let Some(dir) = config_dir {
        paths.push(dir.join("config.toml"));
        paths.push(dir.join("config.yaml"));
        paths.push(dir.join("config.yml"));
        paths.push(dir.join("config.json"));
    }

    paths
}

/// Walk upward from `start_dir` looking for a project-local `.mdfm.toml` file.
pub fn find_local_config(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = Some(start_dir);
    while let Some(current) = dir {
        let candidate = current.join(".mdfm.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = current.parent();
    }
    None
}

/// Fully resolved configuration after evaluating the layered precedence chain:
/// 1. CLI flags
/// 2. `MDFM_*` environment variables
/// 3. Project-local `.mdfm.toml` (or explicit `--config`)
/// 4. User global configuration (`~/.config/mdfm/config.*`)
/// 5. Built-in defaults
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub default_format: Format,
    pub output: OutputFormat,
    pub schema: Option<PathBuf>,
    pub ignore: Vec<String>,
    pub extensions: Vec<String>,
    pub errors_only: bool,
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            default_format: Format::Yaml,
            output: OutputFormat::Table,
            schema: None,
            ignore: Vec::new(),
            extensions: default_extensions(),
            errors_only: false,
        }
    }
}

/// Command-line overrides applied at the top of the configuration precedence chain.
#[derive(Debug, Default, Clone)]
pub struct CliOverrides<'a> {
    pub config_path: Option<&'a Path>,
    pub format: Option<Format>,
    pub output: Option<OutputFormat>,
    pub schema: Option<&'a Path>,
    pub extensions: Option<&'a [String]>,
    pub errors_only: Option<bool>,
}

fn parse_format_env(raw: &str) -> Result<Format> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "yaml" | "yml" => Ok(Format::Yaml),
        "toml" => Ok(Format::Toml),
        other => anyhow::bail!(
            "Invalid frontmatter format '{other}' from MDFM_FORMAT. Expected 'yaml' or 'toml'"
        ),
    }
}

fn parse_output_env(raw: &str) -> Result<OutputFormat> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "table" => Ok(OutputFormat::Table),
        "json" => Ok(OutputFormat::Json),
        "plain" => Ok(OutputFormat::Plain),
        other => anyhow::bail!("Invalid output format '{other}' from MDFM_OUTPUT. Expected 'table', 'json', or 'plain'"),
    }
}

fn resolve_schema_relative_to_config(schema: PathBuf, config_path: &Path) -> PathBuf {
    if schema.is_absolute() {
        return schema;
    }
    if let Some(parent) = config_path.parent() {
        let joined = parent.join(&schema);
        if joined.exists() {
            return joined;
        }
    }
    schema
}

/// Resolve configuration using the current working directory.
pub fn resolve_configuration(overrides: &CliOverrides<'_>) -> Result<ResolvedConfig> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_configuration_from_dir(&cwd, overrides)
}

/// Resolve configuration starting local `.mdfm.toml` discovery from `cwd`.
pub fn resolve_configuration_from_dir(
    cwd: &Path,
    overrides: &CliOverrides<'_>,
) -> Result<ResolvedConfig> {
    // Determine explicit config path from CLI or MDFM_CONFIG env var.
    let explicit_path = overrides.config_path.map(Path::to_path_buf).or_else(|| {
        std::env::var("MDFM_CONFIG")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
    });

    let (local_cfg, global_cfg) = if let Some(path) = explicit_path {
        if !path.exists() {
            anyhow::bail!("Configuration file '{}' does not exist", path.display());
        }
        if !path.is_file() {
            anyhow::bail!(
                "Configuration path '{}' is not a regular file",
                path.display()
            );
        }
        let cfg = ConfigFile::load_from_path(&path)?;
        (Some((path, cfg)), None)
    } else {
        let local = match find_local_config(cwd) {
            Some(path) => {
                let cfg = ConfigFile::load_from_path(&path)?;
                Some((path, cfg))
            }
            None => None,
        };
        let global = ConfigFile::load_global()?;
        (local, global)
    };

    // 1. default_format: CLI > MDFM_FORMAT > local > global > default (Yaml)
    let default_format = if let Some(fmt) = overrides.format {
        fmt
    } else if let Ok(env_fmt) = std::env::var("MDFM_FORMAT") {
        if !env_fmt.trim().is_empty() {
            parse_format_env(&env_fmt)?
        } else {
            local_cfg
                .as_ref()
                .and_then(|(_, c)| c.default_format)
                .or_else(|| global_cfg.as_ref().and_then(|(_, c)| c.default_format))
                .unwrap_or_default()
        }
    } else {
        local_cfg
            .as_ref()
            .and_then(|(_, c)| c.default_format)
            .or_else(|| global_cfg.as_ref().and_then(|(_, c)| c.default_format))
            .unwrap_or_default()
    };

    // 2. output: CLI > MDFM_OUTPUT > local > global > default (Table)
    let output = if let Some(out) = overrides.output {
        out
    } else if let Ok(env_out) = std::env::var("MDFM_OUTPUT") {
        if !env_out.trim().is_empty() {
            parse_output_env(&env_out)?
        } else {
            local_cfg
                .as_ref()
                .and_then(|(_, c)| c.output)
                .or_else(|| global_cfg.as_ref().and_then(|(_, c)| c.output))
                .unwrap_or_default()
        }
    } else {
        local_cfg
            .as_ref()
            .and_then(|(_, c)| c.output)
            .or_else(|| global_cfg.as_ref().and_then(|(_, c)| c.output))
            .unwrap_or_default()
    };

    // 3. schema: CLI > MDFM_SCHEMA > local > global > None
    let schema = if let Some(s) = overrides.schema {
        Some(s.to_path_buf())
    } else if let Ok(env_schema) = std::env::var("MDFM_SCHEMA") {
        let trimmed = env_schema.trim();
        if !trimmed.is_empty() {
            Some(PathBuf::from(trimmed))
        } else {
            None
        }
    } else {
        local_cfg
            .as_ref()
            .and_then(|(path, c)| {
                c.schema
                    .clone()
                    .map(|s| resolve_schema_relative_to_config(s, path))
            })
            .or_else(|| {
                global_cfg.as_ref().and_then(|(path, c)| {
                    c.schema
                        .clone()
                        .map(|s| resolve_schema_relative_to_config(s, path))
                })
            })
    };

    // 4. ignore: local > global > []
    let ignore = local_cfg
        .as_ref()
        .and_then(|(_, c)| c.ignore.clone())
        .or_else(|| global_cfg.as_ref().and_then(|(_, c)| c.ignore.clone()))
        .unwrap_or_default();

    // 5. extensions: CLI > local > global > ["md", "markdown"]
    let extensions = if let Some(exts) = overrides.extensions {
        normalize_extensions(exts)?
    } else if let Some(exts) = local_cfg
        .as_ref()
        .and_then(|(_, c)| c.extensions.as_deref())
        .or_else(|| {
            global_cfg
                .as_ref()
                .and_then(|(_, c)| c.extensions.as_deref())
        })
    {
        normalize_extensions(exts)?
    } else {
        default_extensions()
    };

    // 6. errors_only: CLI > local > global > false
    let errors_only = overrides
        .errors_only
        .or_else(|| local_cfg.as_ref().and_then(|(_, c)| c.errors_only))
        .or_else(|| global_cfg.as_ref().and_then(|(_, c)| c.errors_only))
        .unwrap_or(false);

    Ok(ResolvedConfig {
        default_format,
        output,
        schema,
        ignore,
        extensions,
        errors_only,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        saved: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvGuard {
        fn new(keys: &[&'static str]) -> Self {
            let saved = keys.iter().map(|&k| (k, std::env::var_os(k))).collect();
            Self { saved }
        }

        fn set(&self, key: &str, val: &str) {
            std::env::set_var(key, val);
        }

        fn remove(&self, key: &str) {
            std::env::remove_var(key);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.saved {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn parse_toml_config_file() {
        let content = r#"
default_format = "toml"
output = "json"
schema = ".mdfm-schema.json"
ignore = ["drafts/**", "README.md"]
extensions = ["md", ".cook", "MDX"]
errors_only = true
"#;
        let cfg = ConfigFile::parse(content, ConfigFormat::Toml).unwrap();
        assert_eq!(cfg.default_format, Some(Format::Toml));
        assert_eq!(cfg.output, Some(OutputFormat::Json));
        assert_eq!(cfg.schema, Some(PathBuf::from(".mdfm-schema.json")));
        assert_eq!(
            cfg.ignore,
            Some(vec!["drafts/**".to_string(), "README.md".to_string()])
        );
        assert_eq!(
            cfg.extensions,
            Some(vec![
                "md".to_string(),
                ".cook".to_string(),
                "MDX".to_string()
            ])
        );
        assert_eq!(cfg.errors_only, Some(true));
    }

    #[test]
    fn normalize_extensions_rules() {
        assert_eq!(
            normalize_extensions(&[
                ".COOK".to_string(),
                " mdx ".to_string(),
                "cook".to_string(),
                "..md".to_string(),
            ])
            .unwrap(),
            vec!["cook", "mdx", "md"]
        );
        assert!(normalize_extensions(&[]).is_err());
        assert!(normalize_extensions(&["".to_string()]).is_err());
        assert!(normalize_extensions(&[".".to_string()]).is_err());
    }

    #[test]
    fn parse_yaml_json_and_errors() {
        assert_eq!(
            ConfigFormat::from_path(Path::new("cfg.yaml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("cfg.yml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(Path::new("cfg.json")),
            Some(ConfigFormat::Json)
        );
        assert_eq!(ConfigFormat::from_path(Path::new("cfg.txt")), None);

        let yaml_cfg = ConfigFile::parse(
            "default_format: toml\noutput: plain\nschema: s.json\nignore:\n  - a/**\n",
            ConfigFormat::Yaml,
        )
        .unwrap();
        assert_eq!(yaml_cfg.default_format, Some(Format::Toml));
        assert_eq!(yaml_cfg.output, Some(OutputFormat::Plain));

        let json_cfg = ConfigFile::parse(
            r#"{"default_format":"yaml","output":"json","schema":"s.json","ignore":["b/**"]}"#,
            ConfigFormat::Json,
        )
        .unwrap();
        assert_eq!(json_cfg.default_format, Some(Format::Yaml));
        assert_eq!(json_cfg.output, Some(OutputFormat::Json));

        let dir = tempfile::tempdir().unwrap();
        let bad_ext = dir.path().join("config.txt");
        fs::write(&bad_ext, "hello").unwrap();
        assert!(ConfigFile::load_from_path(&bad_ext).is_err());
        assert!(ConfigFile::load_from_path(&dir.path().join("missing.toml")).is_err());

        // ResolvedConfig defaults and env parsers.
        let def = ResolvedConfig::default();
        assert_eq!(def.default_format, Format::Yaml);
        assert_eq!(def.output, OutputFormat::Table);
        assert!(def.schema.is_none());
        assert!(def.ignore.is_empty());
        assert_eq!(def.extensions, vec!["md", "markdown"]);
        assert!(!def.errors_only);

        assert_eq!(parse_format_env("YAML").unwrap(), Format::Yaml);
        assert_eq!(parse_format_env("yml").unwrap(), Format::Yaml);
        assert_eq!(parse_format_env("toml").unwrap(), Format::Toml);
        assert!(parse_format_env("xml").is_err());

        assert_eq!(parse_output_env("table").unwrap(), OutputFormat::Table);
        assert_eq!(parse_output_env("JSON").unwrap(), OutputFormat::Json);
        assert_eq!(parse_output_env("plain").unwrap(), OutputFormat::Plain);
        assert!(parse_output_env("csv").is_err());

        // resolve_schema_relative_to_config: absolute, existing relative, missing relative.
        let abs = dir.path().join("abs.json");
        let cfg_p = dir.path().join(".mdfm.toml");
        assert_eq!(resolve_schema_relative_to_config(abs.clone(), &cfg_p), abs);
        let rel_existing = dir.path().join("existing.json");
        fs::write(&rel_existing, "{}").unwrap();
        assert_eq!(
            resolve_schema_relative_to_config(PathBuf::from("existing.json"), &cfg_p),
            rel_existing
        );
        assert_eq!(
            resolve_schema_relative_to_config(PathBuf::from("missing.json"), &cfg_p),
            PathBuf::from("missing.json")
        );
    }

    #[test]
    fn explicit_config_and_cli_precedence() {
        let _lock = ENV_LOCK.lock().unwrap();
        let env = EnvGuard::new(&[
            "MDFM_CONFIG",
            "MDFM_FORMAT",
            "MDFM_OUTPUT",
            "MDFM_SCHEMA",
            "XDG_CONFIG_HOME",
        ]);
        env.remove("MDFM_CONFIG");
        env.remove("MDFM_FORMAT");
        env.remove("MDFM_OUTPUT");
        env.remove("MDFM_SCHEMA");

        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("custom.toml");
        fs::write(
            &cfg_path,
            "default_format = \"toml\"\noutput = \"plain\"\nschema = \"schema.json\"\nignore = [\"tmp/**\"]\nextensions = [\".cook\", \"MD\"]\nerrors_only = true\n",
        )
        .unwrap();

        // Explicit config overrides defaults.
        let resolved = resolve_configuration_from_dir(
            dir.path(),
            &CliOverrides {
                config_path: Some(&cfg_path),
                ..CliOverrides::default()
            },
        )
        .unwrap();
        assert_eq!(resolved.default_format, Format::Toml);
        assert_eq!(resolved.output, OutputFormat::Plain);
        assert_eq!(resolved.schema, Some(PathBuf::from("schema.json")));
        assert_eq!(resolved.ignore, vec!["tmp/**".to_string()]);
        assert_eq!(resolved.extensions, vec!["cook", "md"]);
        assert!(resolved.errors_only);

        // CLI flags override explicit config.
        let cli_exts = vec![".MDX".to_string()];
        let resolved_cli = resolve_configuration_from_dir(
            dir.path(),
            &CliOverrides {
                config_path: Some(&cfg_path),
                format: Some(Format::Yaml),
                output: Some(OutputFormat::Json),
                schema: Some(Path::new("override.json")),
                extensions: Some(&cli_exts),
                errors_only: Some(false),
            },
        )
        .unwrap();
        assert_eq!(resolved_cli.default_format, Format::Yaml);
        assert_eq!(resolved_cli.output, OutputFormat::Json);
        assert_eq!(resolved_cli.schema, Some(PathBuf::from("override.json")));
        assert_eq!(resolved_cli.extensions, vec!["mdx"]);
        assert!(!resolved_cli.errors_only);

        // Empty configured extensions list fails validation.
        let bad_ext_cfg = dir.path().join("bad_ext.toml");
        fs::write(&bad_ext_cfg, "extensions = []\n").unwrap();
        assert!(resolve_configuration_from_dir(
            dir.path(),
            &CliOverrides {
                config_path: Some(&bad_ext_cfg),
                ..CliOverrides::default()
            }
        )
        .is_err());

        // Error on nonexistent path or directory passed as explicit config.
        assert!(resolve_configuration_from_dir(
            dir.path(),
            &CliOverrides {
                config_path: Some(&dir.path().join("nope.toml")),
                ..CliOverrides::default()
            }
        )
        .is_err());
        assert!(resolve_configuration_from_dir(
            dir.path(),
            &CliOverrides {
                config_path: Some(dir.path()),
                ..CliOverrides::default()
            }
        )
        .is_err());
    }

    #[test]
    fn env_vars_and_global_config_resolution() {
        let _lock = ENV_LOCK.lock().unwrap();
        let env = EnvGuard::new(&[
            "MDFM_CONFIG",
            "MDFM_FORMAT",
            "MDFM_OUTPUT",
            "MDFM_SCHEMA",
            "XDG_CONFIG_HOME",
            "HOME",
            "APPDATA",
            "USERPROFILE",
        ]);
        for k in [
            "MDFM_CONFIG",
            "MDFM_FORMAT",
            "MDFM_OUTPUT",
            "MDFM_SCHEMA",
            "XDG_CONFIG_HOME",
            "HOME",
            "APPDATA",
            "USERPROFILE",
        ] {
            env.remove(k);
        }

        let dir = tempfile::tempdir().unwrap();
        let xdg = dir.path().join("xdg");
        let global_dir = xdg.join("mdfm");
        fs::create_dir_all(&global_dir).unwrap();
        let global_schema = global_dir.join("global-schema.json");
        fs::write(&global_schema, "{}").unwrap();
        fs::write(
            global_dir.join("config.yaml"),
            "default_format: toml\noutput: plain\nschema: global-schema.json\nignore:\n  - g/**\nextensions:\n  - cook\n  - md\nerrors_only: true\n",
        )
        .unwrap();

        // Global config discovery via XDG_CONFIG_HOME.
        env.set("XDG_CONFIG_HOME", xdg.to_str().unwrap());
        let resolved =
            resolve_configuration_from_dir(dir.path(), &CliOverrides::default()).unwrap();
        assert_eq!(resolved.default_format, Format::Toml);
        assert_eq!(resolved.output, OutputFormat::Plain);
        assert_eq!(resolved.schema, Some(global_schema));
        assert_eq!(resolved.ignore, vec!["g/**".to_string()]);
        assert_eq!(resolved.extensions, vec!["cook", "md"]);
        assert!(resolved.errors_only);

        // Empty/whitespace MDFM_* env vars fall back to config values.
        env.set("MDFM_FORMAT", "   ");
        env.set("MDFM_OUTPUT", " ");
        env.set("MDFM_SCHEMA", " ");
        let resolved_ws =
            resolve_configuration_from_dir(dir.path(), &CliOverrides::default()).unwrap();
        assert_eq!(resolved_ws.default_format, Format::Toml);
        assert_eq!(resolved_ws.output, OutputFormat::Plain);
        assert_eq!(resolved_ws.schema, None);

        // Non-empty MDFM_* env vars override global config.
        env.set("MDFM_FORMAT", "yaml");
        env.set("MDFM_OUTPUT", "json");
        env.set("MDFM_SCHEMA", "env-schema.json");
        let resolved_env =
            resolve_configuration_from_dir(dir.path(), &CliOverrides::default()).unwrap();
        assert_eq!(resolved_env.default_format, Format::Yaml);
        assert_eq!(resolved_env.output, OutputFormat::Json);
        assert_eq!(resolved_env.schema, Some(PathBuf::from("env-schema.json")));

        // MDFM_CONFIG loads explicit JSON config.
        let json_cfg_path = dir.path().join("explicit.json");
        fs::write(
            &json_cfg_path,
            r#"{"default_format":"toml","output":"plain"}"#,
        )
        .unwrap();
        env.remove("MDFM_FORMAT");
        env.remove("MDFM_OUTPUT");
        env.remove("MDFM_SCHEMA");
        env.set("MDFM_CONFIG", json_cfg_path.to_str().unwrap());
        let resolved_cfg =
            resolve_configuration_from_dir(dir.path(), &CliOverrides::default()).unwrap();
        assert_eq!(resolved_cfg.default_format, Format::Toml);
        assert_eq!(resolved_cfg.output, OutputFormat::Plain);
        assert_eq!(resolved_cfg.extensions, vec!["md", "markdown"]);
        assert!(!resolved_cfg.errors_only);

        // default_global_config_paths fallback order: HOME -> APPDATA -> USERPROFILE -> none.
        env.remove("XDG_CONFIG_HOME");
        env.set("HOME", "/tmp/home-test");
        let home_paths = default_global_config_paths();
        assert!(home_paths[0].ends_with(".config/mdfm/config.toml"));

        env.remove("HOME");
        env.set("APPDATA", "/tmp/appdata-test");
        let appdata_paths = default_global_config_paths();
        assert!(appdata_paths[0].ends_with("mdfm/config.toml"));

        env.remove("APPDATA");
        env.set("USERPROFILE", "/tmp/userprofile-test");
        let up_paths = default_global_config_paths();
        assert!(up_paths[0].ends_with(".config/mdfm/config.toml"));

        env.remove("USERPROFILE");
        assert!(default_global_config_paths().is_empty());
    }

    #[test]
    fn local_mdfm_toml_discovered_from_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("posts").join("2026");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            dir.path().join(".mdfm.toml"),
            "default_format = \"toml\"\noutput = \"json\"\n",
        )
        .unwrap();

        let found = find_local_config(&sub).expect("should discover parent .mdfm.toml");
        assert_eq!(found, dir.path().join(".mdfm.toml"));
    }
}
