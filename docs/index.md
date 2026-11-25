# mdfm

![GitHub tag (latest SemVer)](https://img.shields.io/github/v/tag/nicholaswilde/mdfm?style=for-the-badge)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/mdfm/ci.yml?label=ci&style=for-the-badge&branch=main&logo=github-actions)](https://github.com/nicholaswilde/mdfm/actions/workflows/ci.yml)
[![Coveralls](https://img.shields.io/coveralls/github/nicholaswilde/mdfm/main?style=for-the-badge&logo=coveralls)](https://coveralls.io/github/nicholaswilde/mdfm?branch=main)
[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=for-the-badge)](https://github.com/nicholaswilde/mdfm/blob/main/LICENSE)

`jq` for Markdown frontmatter: a fast, single-binary Rust CLI for querying, mutating, deleting, and validating YAML (`---`) and TOML (`+++`) frontmatter across Markdown files without modifying the Markdown body, UTF-8 BOM, or line endings.

!!! warning
    This project is currently in a `v0.X.X` development stage. Features and configurations are subject to change, and breaking changes may be introduced at any time.

## :rocket: TL;DR

### :floppy_disk: Installation

=== "Homebrew"

    ```bash
    brew install nicholaswilde/homebrew-tap/mdfm
    ```

=== "APT"

    ```bash
    # After adding https://nicholaswilde.io/apt (see Installation page)
    sudo apt update && sudo apt install mdfm
    ```

=== "Scoop"

    ```powershell
    scoop bucket add nicholaswilde https://github.com/nicholaswilde/scoop-bucket
    scoop install nicholaswilde/mdfm
    ```

=== "Cargo"

    ```bash
    cargo install --git https://github.com/nicholaswilde/mdfm
    ```

### :gear: Usage

```bash title="Quickstart commands"
# Query a frontmatter key (supports dot paths)
mdfm get author.name docs/

# Filter files with --where and emit JSON
mdfm --where 'draft == true' --output json get title posts/

# Preview updates with --dry-run, then apply in-place
mdfm set draft=false 'tags=["rust","cli"]' posts/ --dry-run
mdfm set draft=false 'tags=["rust","cli"]' posts/

# Delete one or more frontmatter keys
mdfm del draft legacy_id posts/

# Validate frontmatter against a JSON Schema
mdfm check --schema .mdfm-schema.json posts/

# Check for or install the latest GitHub release
mdfm update --check
```

## :star: Contributing

See [Contributing](./CONTRIBUTING.md).

## :sparkles: Code of Conduct

See [Code of Conduct](./CODE_OF_CONDUCT.md).

## :scales: License

[Apache License 2.0][1]

## :pencil: Author

This project was started in 2026 by [Nicholas Wilde].

[Nicholas Wilde]: https://github.com/nicholaswilde/
[1]: https://github.com/nicholaswilde/mdfm/blob/main/LICENSE
