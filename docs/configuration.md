# Configuration

`mdfm` supports optional configuration files in **TOML**, **YAML**, or **JSON** format. A commented template documenting all supported options and defaults is provided in [`assets/config.example.toml`](https://github.com/nicholaswilde/mdfm/blob/main/assets/config.example.toml) (also bundled in release archives as `config.example.toml`).

To initialize a project-local or user-global configuration file:

```bash
# Project-local config (discovered automatically from subdirectories)
cp assets/config.example.toml .mdfm.toml

# Or user-global config
mkdir -p ~/.config/mdfm
cp assets/config.example.toml ~/.config/mdfm/config.toml
```

## :1234: Hierarchical Precedence

Settings resolve in strict hierarchical precedence order (highest to lowest):

1. **Command Line Flags** (`--format`, `--output`, `--schema`, `-C` / `--config`)
2. **Environment Variables** (`MDFM_FORMAT`, `MDFM_OUTPUT`, `MDFM_SCHEMA`, `MDFM_CONFIG`)
3. **Project-Local Configuration** (`.mdfm.toml`, discovered by walking upward from the current working directory — or explicit `-C` / `--config` / `MDFM_CONFIG`)
4. **User Global Configuration** (`$XDG_CONFIG_HOME/mdfm/config.{toml,yaml,yml,json}`, `~/.config/mdfm/config.{toml,yaml,yml,json}`, or `%APPDATA%\mdfm\config.*` / `%USERPROFILE%\.config\mdfm\config.*` on Windows)
5. **Built-In Defaults** (`default_format = "yaml"`, `output = "table"`, `schema = None`, `ignore = []`)

!!! note
    Passing `-C` / `--config <path>` or setting `MDFM_CONFIG` loads only that explicit file (failing immediately if the path does not exist or is not a regular file) and skips automatic local/global discovery.

## :mag: Discovery & Relative Schema Paths

- **Upward `.mdfm.toml` Lookup:** Running `mdfm` inside a nested subdirectory (such as `posts/2026/`) walks parent directories until it locates the nearest `.mdfm.toml`.
- **Global Config Extension Priority:** When probing the user config directory, `mdfm` checks files in order: `config.toml`, `config.yaml`, `config.yml`, `config.json`.
- **Relative `schema` Resolution:** When `schema` is a relative path in a configuration file (for example, `schema = ".mdfm-schema.json"` inside `.mdfm.toml`), `mdfm` resolves it relative to the configuration file's directory if that path exists; otherwise it falls back to the current working directory.

## :gear: Example Configuration Files

A ready-to-use template is available at [`assets/config.example.toml`](https://github.com/nicholaswilde/mdfm/blob/main/assets/config.example.toml).

=== "TOML (`.mdfm.toml` / `config.toml`)"

    ```toml title=".mdfm.toml"
    # Frontmatter format when creating new frontmatter ("yaml" or "toml")
    default_format = "yaml"

    # Default output presentation ("table", "json", or "plain")
    output = "table"

    # Default JSON Schema path for `mdfm check`
    schema = ".mdfm-schema.json"

    # Extra glob patterns excluded during directory walking (in addition to .gitignore)
    ignore = ["drafts/**", "CHANGELOG.md"]
    ```

=== "YAML (`config.yaml`)"

    ```yaml title="~/.config/mdfm/config.yaml"
    default_format: yaml
    output: table
    schema: .mdfm-schema.json
    ignore:
      - drafts/**
      - CHANGELOG.md
    ```

=== "JSON (`config.json`)"

    ```json title="~/.config/mdfm/config.json"
    {
      "default_format": "yaml",
      "output": "table",
      "schema": ".mdfm-schema.json",
      "ignore": [
        "drafts/**",
        "CHANGELOG.md"
      ]
    }
    ```

## :card_index_dividers: Configuration Options Reference

| Option | Type | Default | Supported Values | Description |
| :--- | :--- | :--- | :--- | :--- |
| `default_format` | String | `"yaml"` | `"yaml"`, `"toml"` | Frontmatter format used when `set` creates frontmatter on a body-only file |
| `output` | String | `"table"` | `"table"`, `"json"`, `"plain"` | Output presentation format for `get`, `set`, `del`, and `check` |
| `schema` | String (Path) | None | File path | Default JSON Schema file used by `mdfm check` |
| `ignore` | Array of Strings | `[]` | Glob patterns | Additional file/directory globs excluded during directory traversal |

## :earth_americas: Environment Variables Reference

| Variable | Config Option | CLI Flag | Description |
| :--- | :--- | :--- | :--- |
| `MDFM_FORMAT` | `default_format` | `--format` | Frontmatter format when creating new frontmatter (`yaml`, `yml`, or `toml`) |
| `MDFM_OUTPUT` | `output` | `--output` | Output format (`table`, `json`, or `plain`) |
| `MDFM_SCHEMA` | `schema` | `--schema` | Default JSON Schema path for `mdfm check` |
| `MDFM_CONFIG` | N/A | `-C`, `--config` | Explicit path to a configuration file (`.toml`, `.yaml`, `.yml`, or `.json`) |
