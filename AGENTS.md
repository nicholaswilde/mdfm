# AGENTS.md

Conventions and architecture for AI agents working on `mdfm`.

---

<!-- rtk-begin -->
## RTK Command Guidelines

- **Git Operations**: Prefix `git` commands with `rtk` (e.g., `rtk git status`, `rtk git diff`, `rtk git log`, `rtk git commit`, `rtk git push`).
- **GitHub CLI**: Prefix `gh` commands with `rtk` and pipe to `cat` (e.g., `rtk gh issue list | cat`, `rtk gh pr status | cat`). Always pipe to bypass interactive pagers.
- **File & Directory Inspection**: Use `rtk ls`, `rtk tree`, `rtk fd`, or `rtk read` when listing or reading files for token-optimized output.
- **Searching**: Use `rtk rg` or `rg` (ripgrep) for line search pattern matching to save tokens.
- **TOML Operations**: Prefix `taplo` commands with `rtk` (e.g., `rtk taplo fmt`, `rtk taplo lint`, `rtk taplo check`).
- **Markdown Operations**: Prefix `rumdl` commands with `rtk` (e.g., `rtk rumdl check .`, `rtk rumdl fmt .`).
- **Ripwire Operations**: Prefix `ripwire` commands with `rtk` (e.g., `rtk ripwire <dir> --for="<task>"`). Always invoke via CLI, never register or use MCP.
- **Build & Test Outputs**: Use `rtk err` or `rtk test` when running build/test commands to filter output to errors/failures only (e.g., `rtk test cargo test`).
- **Python & UV Operations**: Prefix Python scripts and commands with `uv run`, and prefix `uv` commands with `rtk`. Never run bare `python` or bare `uv`.
- **Exceptions**: Inside `ctx_execute` / `ctx_batch_execute` sandbox code, skip `rtk` (only printed output enters context). Drop `rtk` when exact raw bytes or patch required (`git diff` patch).
<!-- rtk-end -->

<!-- context-mode-begin -->
## Context-Mode Routing Guidelines

- **Derive, Do Not Dump**: Do NOT use `ctx_execute_file` or `ctx_execute` to dump whole file or full config. Print only specific value, matches, count, or known line range needed.
- **Tool Call Surface**: When using MCP wrappers, call `call_mcp_tool` with `ServerName: "context-mode_context-mode"` (or `"context-mode"`) and `ToolName: "ctx_execute"`, `"ctx_execute_file"`, `"ctx_batch_execute"`, `"ctx_fetch_and_index"`, `"ctx_search"`, or `"ctx_index"`.
<!-- context-mode-end -->

<!-- ponytail-begin -->
## Ponytail (Lazy Senior Dev Mode) Guidelines

- **Stop at first rung that holds**: YAGNI -> reuse existing -> stdlib -> native platform -> installed dep -> one line -> minimum working code.
- **Rules**: No unrequested abstractions, boilerplate, or avoidable dependencies. Deletion over addition. Boring over clever. Fewest files possible.
<!-- ponytail-end -->

<!-- caveman-begin -->
## Caveman Guidelines

Respond terse like smart caveman. All technical substance stay. Only fluff die. Code/commits/PRs written normal.
<!-- caveman-end -->

---

## 1. Project Overview & Architecture

**`mdfm`**: Rust CLI (`jq` for Markdown frontmatter) for querying, mutating, deleting, and validating YAML and TOML frontmatter across Markdown files without altering Markdown bodies, UTF-8 BOM, or line endings.

### Module Layout

- `src/main.rs`: Thin binary wrapper calling `mdfm::run()`.
- `src/lib.rs`: Module exports, CLI dispatch, and `run()` / `run_app()`.
- `src/cli.rs`: `clap` derive/env CLI definitions (`get`, `set`, `del`, `check`).
- `src/config.rs`: Layered configuration precedence (CLI > `MDFM_*` env > `.mdfm.toml` > user global config > defaults).
- `src/frontmatter/`:
  - `mod.rs`: Zero-copy byte-offset splitter, renderer, `FrontmatterData` enum.
  - `yaml.rs`: YAML dot-path get/set/del helpers.
  - `toml.rs`: Lossless TOML dot-path get/set/del via `toml_edit`.
  - `filter.rs`: `--where` expression parser and evaluator.
- `src/commands/`:
  - `mod.rs`: Shared file walker (`ignore`) and output formatters (`comfy-table`, `owo-colors`, JSON, plain).
  - `get.rs`, `set.rs`, `del.rs`, `check.rs`: Subcommand handlers.

## 2. Technology Stack

- **Language & Edition:** Rust (Edition 2021)
- **CLI Parsing:** `clap` (derive, env)
- **Frontmatter:** `toml_edit`, `toml`, `serde`, `serde_json`, `serde_yaml_ng`
- **File Walking:** `ignore`
- **Validation:** `jsonschema` (pure Rust, `default-features = false`)
- **Output:** `comfy-table`, `owo-colors`
- **Error Handling:** `thiserror`, `anyhow`
- **Build & Multi-Arch:** `cross`, `build.rs` (`PROJECT_VERSION`), `task` (`Taskfile.yml`)

## 3. Quality Gates

Before completing any task:

- [ ] `task check` passes.
- [ ] `task fmt:check` (`cargo fmt --all -- --check`) passes.
- [ ] `task clippy` (`cargo clippy --workspace --all-targets --all-features -- -D warnings`) passes.
- [ ] `task test` passes all unit and integration tests.
- [ ] `task lint:md` (`rumdl check .`) passes.
