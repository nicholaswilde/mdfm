# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_mdfm_global_optspecs
    string join \n format= output= l/files-only where= C/config= ext= h/help V/version
end

function __fish_mdfm_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_mdfm_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_mdfm_using_subcommand
    set -l cmd (__fish_mdfm_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c mdfm -n "__fish_mdfm_needs_command" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_needs_command" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_needs_command" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_needs_command" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_needs_command" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_needs_command" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_needs_command" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_needs_command" -s V -l version -d 'Print version'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "get" -d 'Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or \'.\' for full frontmatter)'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "set" -d 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "del" -d 'Delete one or more keys from frontmatter'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "mv" -d 'Rename or merge a frontmatter key across files'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "rename" -d 'Rename or merge a frontmatter key across files'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "check" -d 'Validate frontmatter against a JSON Schema'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "completions" -d 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "man" -d 'Generate UNIX roff man page'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "update" -d 'Check for and install the latest mdfm release from GitHub'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -s k -l key -d 'Frontmatter key(s) to query (repeatable or comma-separated; e.g. -k title -k tags)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l dry-run -d 'Preview changes without writing files to disk'
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l diff -d 'Show unified diff of frontmatter edits'
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l dry-run -d 'Preview deletions without writing files to disk'
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l diff -d 'Show unified diff of frontmatter deletions'
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l merge -d 'Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)'
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l overwrite -d 'Overwrite destination key if it already exists'
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l dry-run -d 'Preview renames without writing files to disk'
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -l diff -d 'Show unified diff of frontmatter edits'
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand mv" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l merge -d 'Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)'
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l overwrite -d 'Overwrite destination key if it already exists'
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l dry-run -d 'Preview renames without writing files to disk'
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -l diff -d 'Show unified diff of frontmatter edits'
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand rename" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l schema -d 'Path to JSON Schema file' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -s q -l errors-only -l quiet -d 'Show only validation failures and a compact summary line'
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand man" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -l version -d 'Target a specific release version or tag (e.g. 0.1.0 or v0.1.0)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -l output -d 'Output presentation format (table, json, plain, or files)' -r -f -a "table\t''
json\t''
plain\t''
files\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'not has_frontmatter\', \'frontmatter_format == "toml"\', \'`prep time` != null\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -l ext -d 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -l check -d 'Check for available updates without installing'
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -s y -l yes -d 'Skip confirmation prompt before installing'
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -s l -l files-only -d 'Print only matching file paths, one per line (shorthand for --output files)'
complete -c mdfm -n "__fish_mdfm_using_subcommand update" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "get" -d 'Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or \'.\' for full frontmatter)'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "set" -d 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "del" -d 'Delete one or more keys from frontmatter'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "mv" -d 'Rename or merge a frontmatter key across files'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "check" -d 'Validate frontmatter against a JSON Schema'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "completions" -d 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "man" -d 'Generate UNIX roff man page'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "update" -d 'Check for and install the latest mdfm release from GitHub'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del mv check completions man update help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
