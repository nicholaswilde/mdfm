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
- **Tool Call Surface**: Call `call_mcp_tool` with `ServerName: "context-mode_context-mode"` (or `"context-mode"`) and `ToolName: "ctx_execute"`, `"ctx_execute_file"`, `"ctx_batch_execute"`, `"ctx_fetch_and_index"`, `"ctx_search"`, or `"ctx_index"`.
- **Plugin MCP Only**: Retain plugin MCP server; never duplicate `context-mode` in `mcp_config.json`.
<!-- context-mode-end -->

<!-- ponytail-begin -->
## Ponytail (Lazy Senior Dev Mode) Guidelines

- **Stop at first rung that holds**: YAGNI -> reuse existing -> stdlib -> native platform -> installed dep -> one line -> minimum working code.
- **Root Cause**: Bug fix = root cause, not symptom. Fix shared path once.
- **Rules**: No unrequested abstractions, boilerplate, or avoidable dependencies. Deletion over addition. Boring over clever. Fewest files possible.
<!-- ponytail-end -->

<!-- caveman-begin -->
## Caveman Guidelines

Respond terse like smart caveman. All technical substance stay. Only fluff die. Code/commits/PRs written normal.
<!-- caveman-end -->

<!-- codegraph-begin -->
## CodeGraph

When `.codegraph/` exists at repo root, reach for it BEFORE grep/find or reading files when exploring code:

- **MCP tool**: `codegraph_explore` returns source + call path + blast radius in one call.
- **Shell**: `codegraph explore "<symbol names or question>"`.
- **Sync**: Run `codegraph sync` after edits; skip CodeGraph if `.codegraph/` missing.
<!-- codegraph-end -->

<!-- serena-begin -->
## Serena Semantic Navigation & LSP Integration

Use Serena (`serena`) MCP tools for semantic navigation, AST symbol inspection, LSP diagnostics, and project memories:

1. **Map first**: Call `get_symbols_overview` on target file instead of reading entire file.
2. **Read target symbol only**: Call `find_symbol` with `include_body: true` for target function/struct/method.
3. **Check callers**: Call `find_referencing_symbols` or `find_implementations` before signature/behavior changes.
4. **Edit by symbol**: Use `replace_symbol_body`, `insert_after_symbol`, `insert_before_symbol`, or `rename_symbol`.
5. **Verify diagnostics**: Call `get_diagnostics_for_file` after edits to catch LSP errors before full builds.
<!-- serena-end -->

<!-- headroom-begin -->
## Headroom MCP (Context Window Compression)

Use Headroom (`headroom`) MCP tools for large payloads:

- `headroom_compress`: Shrink large tool outputs or raw data.
- `headroom_retrieve`: Retrieve original content by hash when specific details needed.
- `headroom_stats`: Monitor headroom cache usage.
<!-- headroom-end -->

<!-- ripwire-begin -->
## Ripwire CLI Guidelines

Use `ripwire` CLI (prefixed with `rtk ripwire`) for AST codebase mapping, call graphs, blast radius, and structural queries:

- **CLI First**: Always invoke `rtk ripwire <dir> [flags]` via CLI; do NOT use MCP server.
- **Task Orientation**: `rtk ripwire <dir> --for="<task>"` for ranked signatures and bodies.
- **Task Bundle**: `rtk ripwire <dir> --pack-task="<task>" --legend=compact`.
- **Stack Trace / Error**: `rtk ripwire <dir> --from-trace=FILE --legend=compact` (`-` for stdin).
- **Call Graph**: `rtk ripwire <dir> --callers=SYM`, `--callees=SYM`, `--uses=SYM`.
- **Blast Radius**: `rtk ripwire <dir> --impact=SYM --legend=compact` before changing shared code.
- **Quality Delta & Gate**: `rtk ripwire <dir> --quality-delta --legend=compact` and `--test-gate --legend=compact`.
- **Atomic Edits**: `rtk ripwire <dir> --replace-symbol-body=SYM --edit-payload=FILE` without whole-file re-reading.
<!-- ripwire-end -->

<!-- uv-rules-begin -->
## Python & UV Rules

Prefix Python scripts and commands with `uv run` (`rtk uv run <script>` in interactive shells). Never invoke bare `python`, `python3`, `pip`, or bare `uv`. Keep `uv.lock` and `pyproject.toml` synchronized whenever Python dependencies or configs change.
<!-- uv-rules-end -->

<!-- ripgrep-guidelines-begin -->
## Ripgrep Guidelines

- **Pattern Searching**: Always use `rg` or `rtk rg` instead of `grep -r`.
- **Scope**: Use `-t rust` for Rust files, `-l` for file listing, small context (`-C 1`).
<!-- ripgrep-guidelines-end -->

<!-- taplo-rules-begin -->
## Taplo TOML Rules

- **Prefix with RTK**: Always prefix `taplo` commands with `rtk` (`rtk taplo fmt`, `rtk taplo lint`).
- **Format & Validate**: Run `rtk taplo fmt` to format TOML files in-place and `rtk taplo lint` to validate syntax/schema.
<!-- taplo-rules-end -->

<!-- issue-management-begin -->
## GitHub Issue Management

- **Use Remote GitHub Issues**: Track features/bugs via `rtk gh issue ... | cat` (no local Conductor tracks).
- **Commit Closing**: GitHub issues only close automatically when commits containing `Fixes #<number>` are pushed to remote.
<!-- issue-management-end -->

<!-- sensitive-information-protection-begin -->
## Sensitive Information & Secret Protection

- **No Secrets**: NEVER commit or stage credentials, API tokens, private keys, or `.env` files.
- **No Blanket Staging**: Never use `git add .` or `git add -A` when untracked secrets exist; stage explicitly by filename.
<!-- sensitive-information-protection-end -->

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
