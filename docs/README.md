# :page_facing_up: mdfm :wrench:

![GitHub tag (latest SemVer)](https://img.shields.io/github/v/tag/nicholaswilde/mdfm?style=for-the-badge)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/mdfm/ci.yml?label=ci&style=for-the-badge&branch=main&logo=github-actions)](https://github.com/nicholaswilde/mdfm/actions/workflows/ci.yml)
[![Coveralls](https://img.shields.io/coveralls/github/nicholaswilde/mdfm/main?style=for-the-badge&logo=coveralls)](https://coveralls.io/github/nicholaswilde/mdfm?branch=main)
[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=for-the-badge)](../LICENSE)

`jq` for Markdown frontmatter: a fast, single-binary Rust CLI for querying, mutating, deleting, and validating YAML (`---`) and TOML (`+++`) frontmatter across Markdown files without modifying the Markdown body, UTF-8 BOM, or line endings.

> [!WARNING]
> This project is currently in a `v0.X.X` development stage. Features and configurations are subject to change, and breaking changes may be introduced at any time.

---

## :rocket:&nbsp; TL;DR

### :floppy_disk:&nbsp; Installation

```bash
# Homebrew
brew install nicholaswilde/homebrew-tap/mdfm
```

```powershell
# Scoop (Windows)
scoop bucket add nicholaswilde https://github.com/nicholaswilde/scoop-bucket
scoop install nicholaswilde/mdfm
```

```bash
# Cargo (from source)
cargo install --git https://github.com/nicholaswilde/mdfm
```

Debian/Ubuntu `.deb` packages are also published via [`nicholaswilde/apt`](https://nicholaswilde.io/apt) — see [Installation](https://nicholaswilde.io/mdfm/installation/).

### :gear:&nbsp; Usage

```bash
# Query a frontmatter key (supports dot paths)
mdfm get author.name docs/

# Filter files with --where and emit JSON
mdfm --where 'draft == true' --output json get title posts/

# Update or insert frontmatter keys (dry-run preview first)
mdfm set draft=false 'tags=["rust","cli"]' posts/ --dry-run
mdfm set draft=false 'tags=["rust","cli"]' posts/

# Delete frontmatter keys
mdfm del draft legacy_id posts/

# Validate frontmatter against a JSON Schema (quiet CI mode)
mdfm check --errors-only --schema .mdfm-schema.json posts/

# Walk custom file extensions (e.g. CookLang or MDX)
mdfm --ext cook get title recipes/
```

---

## :wrench:&nbsp; Configuration

`mdfm` resolves configuration in strict precedence order:

1. CLI flags (`--format`, `--output`, `--schema`, `--ext`, `--errors-only` / `-q`, `--config`)
2. Environment variables (`MDFM_FORMAT`, `MDFM_OUTPUT`, `MDFM_SCHEMA`, `MDFM_CONFIG`)
3. Project-local `.mdfm.toml`
4. User global config (`~/.config/mdfm/config.toml`)
5. Built-in defaults

See [assets/config.example.toml](../assets/config.example.toml) for all supported options.

---

## :book:&nbsp; Documentation

Documentation can be found on the [mdfm documentation site](https://nicholaswilde.io/mdfm).

---

## :star:&nbsp; Contributing

See [Contributing](https://nicholaswilde.io/mdfm/CONTRIBUTING/).

---

## :sparkles:&nbsp; Code of Conduct

See [Code of Conduct](https://nicholaswilde.io/mdfm/CODE_OF_CONDUCT/).

---

## :balance_scale:&nbsp; License

[Apache License 2.0](../LICENSE)

---

## :pencil:&nbsp; Author

This project was started in 2026 by [Nicholas Wilde](https://github.com/nicholaswilde/).
