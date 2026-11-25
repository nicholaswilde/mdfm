pub use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
pub use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::frontmatter::Format;

/// Output presentation format for CLI commands.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Plain,
    Files,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "mdfm")]
#[command(
    author,
    version = env!("PROJECT_VERSION"),
    about = "jq for Markdown frontmatter: query, mutate, delete, and validate YAML and TOML frontmatter",
    long_about = None
)]
pub struct Cli {
    /// Frontmatter format used when creating new frontmatter (yaml or toml)
    #[arg(long, env = "MDFM_FORMAT", global = true)]
    pub format: Option<Format>,

    /// Output presentation format (table, json, plain, or files)
    #[arg(long, env = "MDFM_OUTPUT", global = true)]
    pub output: Option<OutputFormat>,

    /// Print only matching file paths, one per line (shorthand for --output files)
    #[arg(short = 'l', long = "files-only", global = true)]
    pub files_only: bool,

    /// Optional filter expression (e.g. 'draft == true', 'not has_frontmatter', 'frontmatter_format == "toml"', '`prep time` != null')
    #[arg(long = "where", value_name = "EXPR", global = true)]
    pub where_expr: Option<String>,

    /// Path to explicit configuration file override (TOML, YAML, or JSON)
    #[arg(short = 'C', long, env = "MDFM_CONFIG", global = true)]
    pub config: Option<PathBuf>,

    /// File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)
    #[arg(long = "ext", value_name = "EXT", value_delimiter = ',', global = true)]
    pub ext: Vec<String>,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn try_parse_from<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Self as clap::Parser>::try_parse_from(itr)
    }

    pub fn parse() -> Self {
        match Self::try_parse_from(std::env::args_os()) {
            Ok(cli) => cli,
            Err(e) => e.exit(),
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or '.' for full frontmatter)
    Get {
        /// Frontmatter key(s) to query (repeatable or comma-separated; e.g. -k title -k tags)
        #[arg(short = 'k', long = "key", value_name = "KEY", value_delimiter = ',')]
        keys: Vec<String>,
        /// Key selector (e.g. title, author.name, title,tags, or '.') followed by optional file or directory paths (defaults to '.')
        #[arg(value_name = "[KEY] [PATHS...]")]
        args: Vec<String>,
    },
    /// Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)
    Set {
        /// KEY=VALUE, KEY+=VALUE, or KEY-=VALUE mutations followed by optional file or directory paths (defaults to '.')
        #[arg(required = true, value_name = "KEY=VALUE [PATHS...]")]
        args: Vec<String>,
        /// Preview changes without writing files to disk
        #[arg(long)]
        dry_run: bool,
        /// Show unified diff of frontmatter edits
        #[arg(long)]
        diff: bool,
    },
    /// Delete one or more keys from frontmatter
    Del {
        /// Frontmatter key paths to delete followed by optional file or directory paths (defaults to '.')
        #[arg(required = true, value_name = "KEY [PATHS...]")]
        args: Vec<String>,
        /// Preview deletions without writing files to disk
        #[arg(long)]
        dry_run: bool,
        /// Show unified diff of frontmatter deletions
        #[arg(long)]
        diff: bool,
    },
    /// Rename or merge a frontmatter key across files
    #[command(visible_alias = "rename")]
    Mv {
        /// Source dot-separated frontmatter key path (e.g. tag, author.full_name)
        old_key: String,
        /// Destination dot-separated frontmatter key path (e.g. tags, author.name)
        new_key: String,
        /// Files or directories to process (defaults to current directory '.')
        paths: Vec<PathBuf>,
        /// Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)
        #[arg(long, conflicts_with = "overwrite")]
        merge: bool,
        /// Overwrite destination key if it already exists
        #[arg(long, conflicts_with = "merge")]
        overwrite: bool,
        /// Preview renames without writing files to disk
        #[arg(long)]
        dry_run: bool,
        /// Show unified diff of frontmatter edits
        #[arg(long)]
        diff: bool,
    },
    /// Validate frontmatter against a JSON Schema
    Check {
        /// Files or directories to validate (defaults to current directory '.')
        paths: Vec<PathBuf>,
        /// Path to JSON Schema file
        #[arg(long, env = "MDFM_SCHEMA")]
        schema: Option<PathBuf>,
        /// Show only validation failures and a compact summary line
        #[arg(short = 'q', long = "errors-only", visible_alias = "quiet")]
        errors_only: bool,
    },
    /// Generate shell completion scripts (bash, zsh, fish, powershell, elvish)
    Completions {
        /// Target shell
        shell: Shell,
    },
    /// Generate UNIX roff man page
    Man,
    /// Check for and install the latest mdfm release from GitHub
    Update {
        /// Check for available updates without installing
        #[arg(long)]
        check: bool,
        /// Skip confirmation prompt before installing
        #[arg(short = 'y', long)]
        yes: bool,
        /// Target a specific release version or tag (e.g. 0.1.0 or v0.1.0)
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
}

/// Render the UNIX roff man page for `mdfm` into `writer`.
pub fn generate_manpage(writer: &mut dyn std::io::Write) -> anyhow::Result<()> {
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(writer)
        .map_err(|e| anyhow::anyhow!("Failed rendering man page: {e}"))
}
