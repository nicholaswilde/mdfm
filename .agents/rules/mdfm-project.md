# mdfm Project Rules

- **Byte-Exact Body Preservation**: Never use `str::lines()` to split or reconstruct Markdown bodies. Preserve UTF-8 BOM, `LF`/`CRLF` line endings, closing delimiter style (`---` vs `...`), trailing newline presence, and untouched body slices verbatim.
- **Lossless TOML**: Use `toml_edit::DocumentMut` for TOML frontmatter mutations so comments, formatting, and key order survive edits.
- **Pure-Rust Multi-Arch**: Keep `jsonschema` with `default-features = false` and avoid native TLS/C dependencies so `cross` multi-arch builds succeed cleanly.
- **Standard Stack Tools**: Task runner `task` (`Taskfile.yml`), `rumdl`, `taplo`, `[profile.release]` size-optimized (`opt-level = "z"`, `lto = true`, `strip = true`).
