
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
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or ''.'' for full frontmatter)')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)')
            [CompletionResult]::new('del', 'del', [CompletionResultType]::ParameterValue, 'Delete one or more keys from frontmatter')
            [CompletionResult]::new('mv', 'mv', [CompletionResultType]::ParameterValue, 'Rename or merge a frontmatter key across files')
            [CompletionResult]::new('rename', 'rename', [CompletionResultType]::ParameterValue, 'Rename or merge a frontmatter key across files')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Validate frontmatter against a JSON Schema')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate UNIX roff man page')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Check for and install the latest mdfm release from GitHub')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'mdfm;get' {
            [CompletionResult]::new('-k', '-k', [CompletionResultType]::ParameterName, 'Frontmatter key(s) to query (repeatable or comma-separated; e.g. -k title -k tags)')
            [CompletionResult]::new('--key', '--key', [CompletionResultType]::ParameterName, 'Frontmatter key(s) to query (repeatable or comma-separated; e.g. -k title -k tags)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;set' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview changes without writing files to disk')
            [CompletionResult]::new('--diff', '--diff', [CompletionResultType]::ParameterName, 'Show unified diff of frontmatter edits')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;del' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview deletions without writing files to disk')
            [CompletionResult]::new('--diff', '--diff', [CompletionResultType]::ParameterName, 'Show unified diff of frontmatter deletions')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;mv' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('--merge', '--merge', [CompletionResultType]::ParameterName, 'Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)')
            [CompletionResult]::new('--overwrite', '--overwrite', [CompletionResultType]::ParameterName, 'Overwrite destination key if it already exists')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview renames without writing files to disk')
            [CompletionResult]::new('--diff', '--diff', [CompletionResultType]::ParameterName, 'Show unified diff of frontmatter edits')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;rename' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('--merge', '--merge', [CompletionResultType]::ParameterName, 'Merge source into destination when destination already exists (deduplicates arrays, shallow-merges mappings)')
            [CompletionResult]::new('--overwrite', '--overwrite', [CompletionResultType]::ParameterName, 'Overwrite destination key if it already exists')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Preview renames without writing files to disk')
            [CompletionResult]::new('--diff', '--diff', [CompletionResultType]::ParameterName, 'Show unified diff of frontmatter edits')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;check' {
            [CompletionResult]::new('--schema', '--schema', [CompletionResultType]::ParameterName, 'Path to JSON Schema file')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Show only validation failures and a compact summary line')
            [CompletionResult]::new('--errors-only', '--errors-only', [CompletionResultType]::ParameterName, 'Show only validation failures and a compact summary line')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Show only validation failures and a compact summary line')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;completions' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;man' {
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;update' {
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Target a specific release version or tag (e.g. 0.1.0 or v0.1.0)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Frontmatter format used when creating new frontmatter (yaml or toml)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output presentation format (table, json, plain, or files)')
            [CompletionResult]::new('--where', '--where', [CompletionResultType]::ParameterName, 'Optional filter expression (e.g. ''draft == true'', ''not has_frontmatter'', ''frontmatter_format == "toml"'', ''`prep time` != null'')')
            [CompletionResult]::new('-C', '-C ', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to explicit configuration file override (TOML, YAML, or JSON)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'File extensions to include when walking directories (repeatable or comma-separated; defaults to md,markdown)')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Check for available updates without installing')
            [CompletionResult]::new('-y', '-y', [CompletionResultType]::ParameterName, 'Skip confirmation prompt before installing')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Skip confirmation prompt before installing')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('--files-only', '--files-only', [CompletionResultType]::ParameterName, 'Print only matching file paths, one per line (shorthand for --output files)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'mdfm;help' {
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Query one or more frontmatter keys (dot paths, comma-separated, -k/--key, or ''.'' for full frontmatter)')
            [CompletionResult]::new('set', 'set', [CompletionResultType]::ParameterValue, 'Update, insert, or mutate array frontmatter keys (creates frontmatter using default_format if absent)')
            [CompletionResult]::new('del', 'del', [CompletionResultType]::ParameterValue, 'Delete one or more keys from frontmatter')
            [CompletionResult]::new('mv', 'mv', [CompletionResultType]::ParameterValue, 'Rename or merge a frontmatter key across files')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Validate frontmatter against a JSON Schema')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts (bash, zsh, fish, powershell, elvish)')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate UNIX roff man page')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Check for and install the latest mdfm release from GitHub')
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
        'mdfm;help;mv' {
            break
        }
        'mdfm;help;check' {
            break
        }
        'mdfm;help;completions' {
            break
        }
        'mdfm;help;man' {
            break
        }
        'mdfm;help;update' {
            break
        }
        'mdfm;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
