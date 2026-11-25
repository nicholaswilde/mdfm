pub mod cli;
pub mod commands;
pub mod config;
pub mod frontmatter;

use std::io::{self, Write};

pub use cli::{Cli, CommandFactory, Commands, OutputFormat};
pub use config::{CliOverrides, ConfigFile, ConfigFormat, ResolvedConfig};
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
    match &cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "mdfm", out);
            return Ok(());
        }
        Commands::Man => {
            return cli::generate_manpage(out);
        }
        Commands::Update {
            check,
            yes,
            version,
        } => {
            return commands::update::execute(
                commands::update::UpdateOptions {
                    check: *check,
                    yes: *yes,
                    version: version.clone(),
                },
                out,
            );
        }
        _ => {}
    }

    let (cli_schema, cli_errors_only) = match &cli.command {
        Commands::Check {
            schema,
            errors_only,
            ..
        } => (schema.as_deref(), (*errors_only).then_some(true)),
        _ => (None, None),
    };

    let cli_output = if cli.files_only {
        Some(OutputFormat::Files)
    } else {
        cli.output
    };

    let resolved = config::resolve_configuration(&config::CliOverrides {
        config_path: cli.config.as_deref(),
        format: cli.format,
        output: cli_output,
        schema: cli_schema,
        extensions: (!cli.ext.is_empty()).then_some(cli.ext.as_slice()),
        errors_only: cli_errors_only,
    })?;

    let filter = commands::compile_filter(cli.where_expr.as_deref())?;

    match &cli.command {
        Commands::Get { keys, args } => {
            commands::get::execute(keys, args, filter.as_ref(), &resolved, out)
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
        Commands::Completions { .. } | Commands::Man | Commands::Update { .. } => {
            unreachable!("handled before config resolution")
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
        assert!(!cli.files_only);
        assert_eq!(cli.where_expr.as_deref(), Some("draft == true"));
        match cli.command {
            Commands::Get { keys, args } => {
                assert!(keys.is_empty());
                assert_eq!(args, vec!["author.name", "posts/"]);
            }
            _ => panic!("expected get subcommand"),
        }

        let multi_get =
            Cli::try_parse_from(["mdfm", "-l", "get", "-k", "title", "-k", "tags", "docs/"])
                .unwrap();
        assert!(multi_get.files_only);
        match multi_get.command {
            Commands::Get { keys, args } => {
                assert_eq!(keys, vec!["title", "tags"]);
                assert_eq!(args, vec!["docs/"]);
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

        let check_cli = Cli::try_parse_from([
            "mdfm",
            "--ext",
            "cook",
            "--ext",
            "mdx,md",
            "check",
            "docs/",
            "--schema",
            ".mdfm-schema.json",
            "--errors-only",
        ])
        .unwrap();
        assert_eq!(check_cli.ext, vec!["cook", "mdx", "md"]);
        match check_cli.command {
            Commands::Check {
                paths,
                schema,
                errors_only,
            } => {
                assert_eq!(paths, vec![PathBuf::from("docs/")]);
                assert_eq!(schema, Some(PathBuf::from(".mdfm-schema.json")));
                assert!(errors_only);
            }
            _ => panic!("expected check subcommand"),
        }

        let quiet_short = Cli::try_parse_from(["mdfm", "check", "-q", "docs/"]).unwrap();
        assert!(matches!(
            quiet_short.command,
            Commands::Check {
                errors_only: true,
                ..
            }
        ));

        let quiet_alias = Cli::try_parse_from(["mdfm", "check", "--quiet", "docs/"]).unwrap();
        assert!(matches!(
            quiet_alias.command,
            Commands::Check {
                errors_only: true,
                ..
            }
        ));
    }

    #[test]
    fn cli_generates_completions_for_all_shells() {
        for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
            let cli = Cli::try_parse_from(["mdfm", "completions", shell]).unwrap();
            let mut buf = Vec::new();
            run_app(cli, &mut buf).unwrap();
            let script = String::from_utf8(buf).unwrap();
            assert!(
                script.contains("mdfm"),
                "completion script for {shell} should mention mdfm"
            );
        }
    }

    #[test]
    fn cli_generates_manpage() {
        let cli = Cli::try_parse_from(["mdfm", "man"]).unwrap();
        assert!(matches!(cli.command, Commands::Man));
        let mut buf = Vec::new();
        run_app(cli, &mut buf).unwrap();
        let roff = String::from_utf8(buf).unwrap();
        assert!(roff.contains(".TH mdfm"));
        assert!(roff.contains(".SH NAME"));
        assert!(roff.contains(".SH SYNOPSIS"));
    }
}
