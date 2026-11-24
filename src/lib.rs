pub mod cli;
pub mod commands;
pub mod config;
pub mod frontmatter;

use std::io::{self, Write};

pub use cli::{Cli, CommandFactory, Commands, OutputFormat};
pub use config::{ConfigFile, ConfigFormat, ResolvedConfig};
pub use frontmatter::{
    ClosingDelimiter, Format, FrontmatterBlock, FrontmatterData, FrontmatterError, LineEnding,
    SplitDocument,
};

/// Main CLI execution entry point invoked by `src/main.rs`.
pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    run_app(cli, &mut io::stdout())
}

/// Dispatch a parsed `Cli` invocation writing formatted output to `out`.
pub fn run_app(cli: Cli, out: &mut dyn Write) -> anyhow::Result<()> {
    let cli_schema = match &cli.command {
        Commands::Check { schema, .. } => schema.as_deref(),
        _ => None,
    };

    let resolved =
        config::resolve_configuration(cli.config.as_deref(), cli.format, cli.output, cli_schema)?;

    let filter = commands::compile_filter(cli.where_expr.as_deref())?;

    match &cli.command {
        Commands::Get { key, paths } => {
            commands::get::execute(key, paths, filter.as_ref(), &resolved, out)
        }
        Commands::Set { args, dry_run } => {
            commands::set::execute(args, *dry_run, filter.as_ref(), &resolved, out)
        }
        Commands::Del { args, dry_run } => {
            commands::del::execute(args, *dry_run, filter.as_ref(), &resolved, out)
        }
        Commands::Check { paths, .. } => {
            commands::check::execute(paths, filter.as_ref(), &resolved, out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cli_parses_global_flags_and_subcommands() {
        let cli = Cli::try_parse_from([
            "mdfm",
            "--format",
            "toml",
            "--output",
            "json",
            "--where",
            "draft == true",
            "get",
            "author.name",
            "posts/",
        ])
        .unwrap();

        assert_eq!(cli.format, Some(Format::Toml));
        assert_eq!(cli.output, Some(OutputFormat::Json));
        assert_eq!(cli.where_expr.as_deref(), Some("draft == true"));
        match cli.command {
            Commands::Get { key, paths } => {
                assert_eq!(key, "author.name");
                assert_eq!(paths, vec![PathBuf::from("posts/")]);
            }
            _ => panic!("expected get subcommand"),
        }
    }

    #[test]
    fn cli_parses_set_del_check() {
        let set_cli =
            Cli::try_parse_from(["mdfm", "set", "draft=false", "post.md", "--dry-run"]).unwrap();
        match set_cli.command {
            Commands::Set { args, dry_run } => {
                assert_eq!(args, vec!["draft=false", "post.md"]);
                assert!(dry_run);
            }
            _ => panic!("expected set subcommand"),
        }

        let del_cli =
            Cli::try_parse_from(["mdfm", "del", "--dry-run", "draft", "post.md"]).unwrap();
        match del_cli.command {
            Commands::Del { args, dry_run } => {
                assert_eq!(args, vec!["draft", "post.md"]);
                assert!(dry_run);
            }
            _ => panic!("expected del subcommand"),
        }

        let check_cli =
            Cli::try_parse_from(["mdfm", "check", "docs/", "--schema", ".mdfm-schema.json"])
                .unwrap();
        match check_cli.command {
            Commands::Check { paths, schema } => {
                assert_eq!(paths, vec![PathBuf::from("docs/")]);
                assert_eq!(schema, Some(PathBuf::from(".mdfm-schema.json")));
            }
            _ => panic!("expected check subcommand"),
        }
    }
}
