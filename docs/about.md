# About

`mdfm` is a fast, single-binary Rust CLI (`jq` for Markdown frontmatter) built for querying, updating, deleting, and validating YAML (`---`) and TOML (`+++`) frontmatter across Markdown collections without touching Markdown bodies, UTF-8 BOMs, or line endings.

## :sparkles: Features

- **Zero-Copy Byte-Offset Splitting:** Splits documents on raw byte offsets instead of rewriting the whole file. The Markdown body slice, UTF-8 BOM (`\u{FEFF}`), `LF`/`CRLF` line endings, closing delimiters (`---`, `...`, `+++`), and trailing-newline presence survive edits verbatim.
- **YAML & Lossless TOML Frontmatter:** Supports both YAML (`---` ... `---`/`...`) and TOML (`+++` ... `+++`) blocks. TOML frontmatter is mutated losslessly via [`toml_edit`][1], preserving comments, whitespace, and key ordering.
- **Dot-Path Queries & Mutations:** Read, insert, update, or delete nested fields using dot-separated paths (such as `author.name` or `seo.canonical_url`), automatically creating intermediate mappings when setting new nested keys.
- **Expressive `--where` Filtering:** Filter files across `get`, `set`, `del`, and `check` using boolean logic (`and`/`&&`, `or`/`||`, `not`/`!`, parentheses), comparisons (`==`, `!=`, `>`, `>=`, `<`, `<=`), array/string `contains`, and truthiness checks.
- **Typed Value Parsing:** `mdfm set` automatically parses booleans, integers, floats, `null`, JSON arrays/objects, and explicitly quoted strings, falling back cleanly to plain strings.
- **Safe Dry-Run Previews:** Preview bulk `set` and `del` mutations with `--dry-run` before writing a single byte to disk.
- **Pure-Rust JSON Schema Validation:** Validate frontmatter contracts across directories and CI pipelines with `mdfm check --schema` powered by [`jsonschema`][2] (compiled with `default-features = false` for deterministic offline execution).
- **`.gitignore`-Aware Directory Walking:** Recursively discovers `.md` and `.markdown` files via the [`ignore`][3] crate while honoring `.gitignore` rules and custom `ignore` globs in configuration.
- **Flexible Output Formats:** Emit TTY-aware Unicode tables (`table`), structured `json` for downstream tools like `jq`, or tab-delimited `plain` output for shell scripts.
- **Layered Configuration:** Resolves defaults cleanly from CLI flags, `MDFM_*` environment variables, upward-discovered `.mdfm.toml` files, and user global config (`~/.config/mdfm/config.{toml,yaml,yml,json}`).
- **Shell Completions:** Generate completion scripts for Bash, Zsh, Fish, PowerShell, and Elvish via `mdfm completions`.
- **Lightweight & Portable:** Single static binary compiled for Linux (`amd64`, `arm64`, `armv7`, `armv6`), macOS (`x86_64`, `arm64`), and Windows (`x86_64`, `i686`).

## :shield: Design & Preservation Guarantees

- **Body Untouched Verbatim:** `mdfm` never parses or reformats the Markdown body after the closing frontmatter delimiter.
- **Horizontal Rules Stay Safe:** An unclosed opening `---` (such as a Markdown thematic break at the top of a file) is treated as body-only content rather than malformed frontmatter.
- **No-Op Write Avoidance:** `set` and `del` skip writing files whose rendered output is byte-identical to the original content. See [Security](./security.md) and [Configuration](./configuration.md) for details.

[1]: https://github.com/toml-rs/toml
[2]: https://github.com/Stranger6667/jsonschema
[3]: https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore
