
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
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand get 'Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or ''.'' for full frontmatter)'
            cand set 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)'
            cand del 'Delete one or more keys from frontmatter'
            cand mv 'Rename or merge a frontmatter key across files'
            cand rename 'Rename or merge a frontmatter key across files'
            cand check 'Validate frontmatter syntax and optionally against a JSON Schema'
            cand keys 'Audit frontmatter keys, file counts, and observed types across files'
            cand values 'Audit unique values and occurrence counts for a frontmatter key across files'
            cand fmt 'Format and normalize frontmatter style (indentation, key/array sorting, YAML/TOML conversion)'
            cand completions 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
            cand man 'Generate UNIX roff man page'
            cand update 'Check for and install the latest mdfm release from GitHub'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'mdfm;get'= {
            cand -k 'Frontmatter key(s) to query (repeatable or comma-separated; e.g. -k title -k tags)'
            cand --key 'Frontmatter key(s) to query (repeatable or comma-separated; e.g. -k title -k tags)'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;set'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --dry-run 'Preview changes without writing files to disk'
            cand --diff 'Show unified diff of frontmatter edits'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;del'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --dry-run 'Preview deletions without writing files to disk'
            cand --diff 'Show unified diff of frontmatter deletions'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;mv'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --merge 'Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)'
            cand --overwrite 'Overwrite destination key if it already exists'
            cand --dry-run 'Preview renames without writing files to disk'
            cand --diff 'Show unified diff of frontmatter edits'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;rename'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --merge 'Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)'
            cand --overwrite 'Overwrite destination key if it already exists'
            cand --dry-run 'Preview renames without writing files to disk'
            cand --diff 'Show unified diff of frontmatter edits'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;check'= {
            cand --schema 'Path to JSON Schema file'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand -q 'Show only validation failures and a compact summary line'
            cand --errors-only 'Show only validation failures and a compact summary line'
            cand --quiet 'Show only validation failures and a compact summary line'
            cand --syntax-only 'Validate YAML/TOML frontmatter syntax only (bypasses any configured JSON Schema)'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;keys'= {
            cand --sort 'Sort order for keys (count descending or alphabetical)'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'mdfm;values'= {
            cand --sort 'Sort order for values (count descending or alphabetical)'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --no-flatten 'Do not flatten array values into individual elements'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'mdfm;fmt'= {
            cand --sort-array 'Sort and deduplicate array values at the given key path(s) (repeatable or comma-separated)'
            cand --to 'Convert frontmatter to the target format (yaml or toml)'
            cand --indent-sequences 'Indent YAML sequence items under their parent key (true or false)'
            cand --yaml-indent 'Number of spaces per YAML indentation level (default: 2)'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --check 'Check formatting without modifying files; exit non-zero if any file needs formatting'
            cand --sort-keys 'Sort frontmatter keys alphabetically (recursively)'
            cand --dry-run 'Preview changes without writing files to disk'
            cand --diff 'Show unified diff of frontmatter formatting changes'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;completions'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;man'= {
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;update'= {
            cand --version 'Target a specific release version or tag (e.g. 0.1.0 or v0.1.0)'
            cand --format 'Frontmatter format used when creating new frontmatter (yaml or toml)'
            cand --output 'Output presentation format (table, json, plain, or files)'
            cand --where 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')'
            cand -C 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --config 'Path to explicit configuration file override (TOML, YAML, or JSON)'
            cand --ext 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)'
            cand --check 'Check for available updates without installing'
            cand -y 'Skip confirmation prompt before installing'
            cand --yes 'Skip confirmation prompt before installing'
            cand -l 'Print only matching file paths, one per line (shorthand for --output files)'
            cand --files-only 'Print only matching file paths, one per line (shorthand for --output files)'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'mdfm;help'= {
            cand get 'Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or ''.'' for full frontmatter)'
            cand set 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)'
            cand del 'Delete one or more keys from frontmatter'
            cand mv 'Rename or merge a frontmatter key across files'
            cand check 'Validate frontmatter syntax and optionally against a JSON Schema'
            cand keys 'Audit frontmatter keys, file counts, and observed types across files'
            cand values 'Audit unique values and occurrence counts for a frontmatter key across files'
            cand fmt 'Format and normalize frontmatter style (indentation, key/array sorting, YAML/TOML conversion)'
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
        &'mdfm;help;mv'= {
        }
        &'mdfm;help;check'= {
        }
        &'mdfm;help;keys'= {
        }
        &'mdfm;help;values'= {
        }
        &'mdfm;help;fmt'= {
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
