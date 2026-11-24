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

/// Representation of values loaded from a configuration file (`.mdfm.toml` or global config).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigFile {
    pub default_format: Option<Format>,
    pub output: Option<OutputFormat>,
    pub schema: Option<PathBuf>,
    pub ignore: Option<Vec<String>>,
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
}

impl Default for ResolvedConfig {
    fn default() -> Self {
        Self {
            default_format: Format::Yaml,
            output: OutputFormat::Table,
            schema: None,
            ignore: Vec::new(),
        }
    }
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
pub fn resolve_configuration(
    cli_config_path: Option<&Path>,
    cli_format: Option<Format>,
    cli_output: Option<OutputFormat>,
    cli_schema: Option<&Path>,
) -> Result<ResolvedConfig> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    resolve_configuration_from_dir(&cwd, cli_config_path, cli_format, cli_output, cli_schema)
}

/// Resolve configuration starting local `.mdfm.toml` discovery from `cwd`.
pub fn resolve_configuration_from_dir(
    cwd: &Path,
    cli_config_path: Option<&Path>,
    cli_format: Option<Format>,
    cli_output: Option<OutputFormat>,
    cli_schema: Option<&Path>,
) -> Result<ResolvedConfig> {
    // Determine explicit config path from CLI or MDFM_CONFIG env var.
    let explicit_path = cli_config_path.map(Path::to_path_buf).or_else(|| {
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
    let default_format = if let Some(fmt) = cli_format {
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
    let output = if let Some(out) = cli_output {
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
    let schema = if let Some(s) = cli_schema {
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

    Ok(ResolvedConfig {
        default_format,
        output,
        schema,
        ignore,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_config_file() {
        let content = r#"
default_format = "toml"
output = "json"
schema = ".mdfm-schema.json"
ignore = ["drafts/**", "README.md"]
"#;
        let cfg = ConfigFile::parse(content, ConfigFormat::Toml).unwrap();
        assert_eq!(cfg.default_format, Some(Format::Toml));
        assert_eq!(cfg.output, Some(OutputFormat::Json));
        assert_eq!(cfg.schema, Some(PathBuf::from(".mdfm-schema.json")));
        assert_eq!(
            cfg.ignore,
            Some(vec!["drafts/**".to_string(), "README.md".to_string()])
        );
    }

    #[test]
    fn explicit_config_and_cli_precedence() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("custom.toml");
        fs::write(
            &cfg_path,
            "default_format = \"toml\"\noutput = \"plain\"\nschema = \"schema.json\"\nignore = [\"tmp/**\"]\n",
        )
        .unwrap();

        // Explicit config overrides defaults.
        let resolved =
            resolve_configuration_from_dir(dir.path(), Some(&cfg_path), None, None, None).unwrap();
        assert_eq!(resolved.default_format, Format::Toml);
        assert_eq!(resolved.output, OutputFormat::Plain);
        assert_eq!(resolved.schema, Some(PathBuf::from("schema.json")));
        assert_eq!(resolved.ignore, vec!["tmp/**".to_string()]);

        // CLI flags override explicit config.
        let resolved_cli = resolve_configuration_from_dir(
            dir.path(),
            Some(&cfg_path),
            Some(Format::Yaml),
            Some(OutputFormat::Json),
            Some(Path::new("override.json")),
        )
        .unwrap();
        assert_eq!(resolved_cli.default_format, Format::Yaml);
        assert_eq!(resolved_cli.output, OutputFormat::Json);
        assert_eq!(resolved_cli.schema, Some(PathBuf::from("override.json")));
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
