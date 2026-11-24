
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'mdfm' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'mdfm'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'mdfm' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, or plain)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''tags contains "rust"'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Query a frontmatter key (supports dot paths like author.name) across Markdown files')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update or insert frontmatter keys (creates frontmatter using default_format if absent)')
            [CompletionResult]::new('del', 'del', [CompletionResultType]::ParameterValue, 'Delete one or more keys from frontmatter')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Validate frontmatter against a JSON Schema')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'mdfm;get' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, or plain)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''tags contains "rust"'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;set' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, or plain)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''tags contains "rust"'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview changes without writing files to disk')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;del' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, or plain)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''tags contains "rust"'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview deletions without writing files to disk')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;check' {
            [CompletionResult]::new('--schema', '--schema', [CompletionResultType]::ParameterName, 'Path to JSON Schema file')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, or plain)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''tags contains "rust"'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;completions' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, or plain)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''author.name == "Nick"'', ''tags contains "rust"'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;help' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Query a frontmatter key (supports dot paths like author.name) across Markdown files')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update or insert frontmatter keys (creates frontmatter using default_format if absent)')
            [CompletionResult]::new('del', 'del', [CompletionResultType]::ParameterValue, 'Delete one or more keys from frontmatter')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Validate frontmatter against a JSON Schema')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'mdfm;help;get' {
            break
        }
        'mdfm;help;set' {
            break
        }
        'mdfm;help;del' {
            break
        }
        'mdfm;help;check' {
            break
        }
        'mdfm;help;completions' {
            break
        }
        'mdfm;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
