
use builtin;
use str;

set edit:completion:arg-completer[mdfm] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'mdfm'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'mdfm'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand get 'Query a frontmatter key (supports dot paths like author.name) across Markdown files'
            cand set 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)'
            cand del 'Delete one or more keys from frontmatter'
            cand check 'Validate frontmatter against a JSON Schema'
            cand completions 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
            cand man 'Generate UNIX roff man page'
            cand update 'Check for and install the latest mdfm release from GitHub'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'mdfm;get'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;set'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --dry-run 'Preview changes without writing files to disk'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;del'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --dry-run 'Preview deletions without writing files to disk'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;check'= {
            cand --schema 'Path to JSON Schema file'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;completions'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;man'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;update'= {
            cand --version 'Target a specific release version or tag (e.g. 0.1.0 or v0.1.0)'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, or plain)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''`prep time` != null'', ''meta["prep time"] == "5m"'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --check 'Check for available updates without installing'
            cand -y 'Skip confirmation prompt before installing'
            cand --yes 'Skip confirmation prompt before installing'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;help'= {
            cand get 'Query a frontmatter key (supports dot paths like author.name) across Markdown files'
            cand set 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)'
            cand del 'Delete one or more keys from frontmatter'
            cand check 'Validate frontmatter against a JSON Schema'
            cand completions 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
            cand man 'Generate UNIX roff man page'
            cand update 'Check for and install the latest mdfm release from GitHub'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'mdfm;help;get'= {
        }
        &'mdfm;help;set'= {
        }
        &'mdfm;help;del'= {
        }
        &'mdfm;help;check'= {
        }
        &'mdfm;help;completions'= {
        }
        &'mdfm;help;man'= {
        }
        &'mdfm;help;update'= {
        }
        &'mdfm;help;help'= {
        }
    ]
    $completions[$command]
}
