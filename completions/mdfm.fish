# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_mdfm_global_optspecs
    string join \n format= output= where= C/config= h/help V/version
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
complete -c mdfm -n "__fish_mdfm_needs_command" -l output -d 'Output presentation format (table, json, or plain)' -r -f -a "table\t''
json\t''
plain\t''"
complete -c mdfm -n "__fish_mdfm_needs_command" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'author.name == "Nick"\', \'tags contains "rust"\')' -r
complete -c mdfm -n "__fish_mdfm_needs_command" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_needs_command" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_needs_command" -s V -l version -d 'Print version'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "get" -d 'Query a frontmatter key (supports dot paths like author.name) across Markdown files'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "set" -d 'Update or insert frontmatter keys (creates frontmatter using default_format if absent)'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "del" -d 'Delete one or more keys from frontmatter'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "check" -d 'Validate frontmatter against a JSON Schema'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "completions" -d 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
complete -c mdfm -n "__fish_mdfm_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l output -d 'Output presentation format (table, json, or plain)' -r -f -a "table\t''
json\t''
plain\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'author.name == "Nick"\', \'tags contains "rust"\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand get" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l output -d 'Output presentation format (table, json, or plain)' -r -f -a "table\t''
json\t''
plain\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'author.name == "Nick"\', \'tags contains "rust"\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -l dry-run -d 'Preview changes without writing files to disk'
complete -c mdfm -n "__fish_mdfm_using_subcommand set" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l output -d 'Output presentation format (table, json, or plain)' -r -f -a "table\t''
json\t''
plain\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'author.name == "Nick"\', \'tags contains "rust"\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -l dry-run -d 'Preview deletions without writing files to disk'
complete -c mdfm -n "__fish_mdfm_using_subcommand del" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l schema -d 'Path to JSON Schema file' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l output -d 'Output presentation format (table, json, or plain)' -r -f -a "table\t''
json\t''
plain\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'author.name == "Nick"\', \'tags contains "rust"\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand check" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l format -d 'Frontmatter format used when creating new frontmatter (yaml or toml)' -r -f -a "yaml\t''
toml\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l output -d 'Output presentation format (table, json, or plain)' -r -f -a "table\t''
json\t''
plain\t''"
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -l where -d 'Optional filter expression (e.g. \'draft == true\', \'author.name == "Nick"\', \'tags contains "rust"\')' -r
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -s C -l config -d 'Path to explicit configuration file override (TOML, YAML, or JSON)' -r -F
complete -c mdfm -n "__fish_mdfm_using_subcommand completions" -s h -l help -d 'Print help'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del check completions help" -f -a "get" -d 'Query a frontmatter key (supports dot paths like author.name) across Markdown files'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del check completions help" -f -a "set" -d 'Update or insert frontmatter keys (creates frontmatter using default_format if absent)'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del check completions help" -f -a "del" -d 'Delete one or more keys from frontmatter'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del check completions help" -f -a "check" -d 'Validate frontmatter against a JSON Schema'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del check completions help" -f -a "completions" -d 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)'
complete -c mdfm -n "__fish_mdfm_using_subcommand help; and not __fish_seen_subcommand_from get set del check completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
