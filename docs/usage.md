# Usage

```bash
mdfm [OPTIONS] <COMMAND>
```

## :keyboard: Commands

| Command | Description | Example |
| :--- | :--- | :--- |
| `get` | Query a frontmatter key (supports dot paths) across Markdown files | `mdfm get author.name docs/` |
| `set` | Update or insert frontmatter keys (creates frontmatter if absent) | `mdfm set draft=false posts/ --dry-run` |
| `del` | Delete one or more keys from frontmatter | `mdfm del draft author.bio posts/` |
| `check` | Validate frontmatter against a JSON Schema | `mdfm check --schema schema.json posts/` |
| `completions` | Generate shell completion script (`bash`, `zsh`, `fish`, `powershell`, `elvish`) | `mdfm completions zsh` |
| `man` | Output UNIX roff man page | `mdfm man > assets/mdfm.1` |
| `update` | Check for and install the latest `mdfm` release from GitHub | `mdfm update [--check] [-y] [--version <ver>]` |

## :gear: Global Options

Global flags may be placed before or after the subcommand:

| Flag | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--format <yaml\|toml>` | `MDFM_FORMAT` | `yaml` | Frontmatter format used when creating new frontmatter on files without any |
| `--output <table\|json\|plain>` | `MDFM_OUTPUT` | `table` | Output presentation format |
| `--where <EXPR>` | N/A | None | Filter files by evaluating a frontmatter predicate expression |
| `-C`, `--config <PATH>` | `MDFM_CONFIG` | Auto-discover | Explicit configuration file path (`.toml`, `.yaml`, `.yml`, or `.json`) |
| `-h`, `--help` | N/A | N/A | Print command help |
| `-V`, `--version` | N/A | N/A | Print `mdfm` version |

## :mag: Querying Keys (`get`)

Query top-level or nested keys across a single file, multiple paths, or a directory (defaults to `.` when no paths are passed):

```bash
# Query a top-level key across a directory
mdfm get title docs/

# Query a nested key using dot notation
mdfm get author.name posts/

# Query a single file in plain format (emits only the value)
mdfm --output plain get title posts/hello.md

# Emit structured JSON for piping into jq
mdfm --output json get tags posts/ | jq '.[].value'
```

Files without frontmatter or without the requested key are silently skipped.

## :pencil2: Updating & Inserting Keys (`set`)

Pass one or more `KEY=VALUE`, `KEY+=VALUE`, or `KEY-=VALUE` mutations followed by optional file or directory paths (defaults to `.`):

```bash
# Preview changes without modifying files on disk
mdfm set draft=false 'tags=["rust","cli"]' posts/ --dry-run

# Apply updates in-place
mdfm set draft=false 'tags=["rust","cli"]' posts/

# Append or remove items from array keys (deduplicated)
mdfm set tags+=must-try docs/breads/savory-star-bread.md
mdfm set tags-=draft docs/
mdfm --where 'tags contains "applied-kitchen"' set tags+=featured docs/

# Set nested keys (intermediate objects/tables are created automatically)
mdfm set author.name="Nicholas Wilde" author.role=maintainer posts/hello.md

# Add frontmatter to body-only Markdown files using TOML format
mdfm --format toml set title="New Note" draft=true notes/plain.md
```

### Array Append & Remove (`+=` / `-=`)

Mutate YAML sequences and TOML arrays without overwriting the entire list:

- **`KEY+=VALUE`**: Appends `VALUE` if not already present (deduplicated). Creates intermediate objects/tables and a new array if `KEY` is absent. Passing a JSON array (e.g. `'tags+=["a","b"]'`) appends each item in order.
- **`KEY-=VALUE`**: Removes matching item(s) while preserving remaining element order. Passing a JSON array (e.g. `'tags-=["draft","wip"]'`) removes each matching item. Missing keys, empty arrays, and body-only files are left untouched as no-ops.
- **Type safety**: Applying `+=` or `-=` to an existing non-array key (such as a string, boolean, number, or table) exits with an error.

### Typed Value Parsing

`mdfm set` infers JSON-compatible types automatically from each `VALUE`:

| Input Example | Parsed Type | Notes |
| :--- | :--- | :--- |
| `draft=false`, `published=true` | Boolean | Exact lowercase `true` or `false` |
| `subtitle=null` | Null | Supported in YAML frontmatter |
| `weight=10`, `offset=-3` | Integer | Parsed as signed 64-bit integer |
| `rating=4.5` | Float | Finite decimal numbers (`inf`/`nan` stay strings) |
| `'tags=["rust","cli"]'` | Array | Valid JSON array starting with `[` |
| `'meta={"pinned":true}'` | Object / Table | Valid JSON object starting with `{` |
| `'version="1.0"'` | String (forced) | Valid JSON quoted string preserves numeric/boolean text as string |
| `title="Hello World"` | String | Default fallback for plain text |

!!! note
    When editing existing frontmatter, `mdfm` preserves its original format (YAML or TOML) and closing delimiter (`---`, `...`, or `+++`). Files whose frontmatter values are semantically unchanged are skipped without rewriting, keeping existing YAML/TOML formatting intact. `--format` / `default_format` only applies when creating frontmatter on a file that has none, emitting a blank line after the closing delimiter before non-empty body content (`rumdl` `MD071`-compatible).

## :wastebasket: Deleting Keys (`del`)

Pass one or more key paths followed by optional file or directory paths (defaults to `.`):

```bash
# Preview deletions without modifying files
mdfm del draft legacy_id posts/ --dry-run

# Delete top-level and nested keys
mdfm del draft author.bio posts/

# Delete keys only from files matching a --where filter
mdfm --where 'draft == false' del reviewer_notes posts/
```

The first positional argument is always parsed as a key path. Subsequent arguments remain keys until an argument exists on disk or resembles a filesystem path (`.`, `..`, contains `/` or `\`, or ends with `.md`/`.markdown`). Files without frontmatter are skipped.

## :mag_right: Filtering Files (`--where`)

The global `--where` flag filters Markdown files before `get`, `set`, `del`, or `check` processes them.

### Supported Operators

| Category | Operators | Examples |
| :--- | :--- | :--- |
| Equality | `==`, `!=` | `draft == true`, `author.name != "Guest"` |
| Ordering | `>`, `>=`, `<`, `<=` | `weight >= 10`, `date >= "2026-01-01"` |
| Membership / Substring | `contains` (or `CONTAINS`) | `tags contains "rust"`, `title contains "Guide"` |
| Logical | `and`/`&&`, `or`/`||`, `not`/`!` | `draft == false && tags contains "rust"` |
| Grouping | `( ... )` | `(draft == false or weight == 10) and not archived` |
| Truthiness | Bare key path | `draft`, `not archived`, `` `prep time` `` |
| Keys with spaces / punctuation | `"..."`, `'...'`, `` `...` ``, `["..."]`, `[N]` | `"prep time" != null`, `` `cook time` ``, `meta["prep time"] == "5 minutes"`, `tags[0] == "rust"` |

### Keys with Spaces or Special Characters

Frontmatter keys containing spaces, dots, punctuation, or filter keywords (such as `and`, `true`, `null`) can be referenced using any of three equivalent forms:

- **Quoted segments:** `"prep time" != null`, `'cook time' == "20 minutes"`, `meta."prep time" == "5 minutes"`
- **Backtick identifiers:** `` `prep time` != null ``, `` meta.`prep time` == "5 minutes" ``
- **Bracket notation:** `["prep time"] != null`, `meta["prep time"] == "5 minutes"`, `["recipe info"]["prep time"]`, `tags[0] == "rust"`

Quoted, backticked, and bracketed segments treat embedded `.` characters as literal parts of the key name (for example, `["a.b"].c` looks up key `"a.b"` followed by `"c"`).

### Filter Examples

```bash
# Find all published posts tagged with "rust"
mdfm --where 'draft == false and tags contains "rust"' get title posts/

# Check array element by index or substring in title
mdfm --where 'tags.0 == "rust" or title contains "Frontmatter"' get title posts/

# Filter on keys containing spaces or special characters
mdfm --where '"prep time" != null' get title recipes/
mdfm --where '`cook time` == "20 minutes" and meta["prep time"] == "5 minutes"' get title recipes/

# Bulk publish drafts with weight >= 10
mdfm --where 'draft == true && weight >= 10' set draft=false posts/ --dry-run

# Validate only non-draft posts against a strict publishing schema
mdfm --where 'not draft' check --schema publish-schema.json posts/
```

!!! tip
    Bare key paths evaluate as falsy when missing, `null`, `false`, `0`, `""`, `[]`, or `{}`. Inequality (`!=`) evaluates to `true` when the referenced key is absent.

## :white_check_mark: Schema Validation (`check`)

Validate YAML and TOML frontmatter against any standard JSON Schema file (such as Draft 2020-12). `mdfm check` exits with a non-zero status code when any file fails validation, making it ready for CI pipelines and pre-commit hooks.

```json title=".mdfm-schema.json"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["title", "draft"],
  "properties": {
    "title": { "type": "string", "minLength": 1 },
    "draft": { "type": "boolean" },
    "weight": { "type": "integer" },
    "tags": {
      "type": "array",
      "items": { "type": "string" }
    },
    "author": {
      "type": "object",
      "properties": {
        "name": { "type": "string" },
        "role": { "type": "string" }
      }
    }
  }
}
```

```bash
# Validate a directory with an explicit schema
mdfm check --schema .mdfm-schema.json docs/

# Emit machine-readable validation results as JSON
mdfm --output json check --schema .mdfm-schema.json docs/

# Rely on `schema = ".mdfm-schema.json"` in .mdfm.toml or MDFM_SCHEMA
mdfm check docs/
```

Files without frontmatter are validated as an empty JSON object (`{}`) unless filtered out by `--where`. Malformed YAML or TOML frontmatter is reported as a validation failure for that file.

## :bar_chart: Output Formats

All subcommands support `--output table` (default), `--output json`, and `--output plain`:

=== "Table"

    ```text
    ┌────────────────────┬───────────────────┐
    │ File               ┆ title             │
    ╞════════════════════╪═══════════════════╡
    │ posts/toml_post.md ┆ TOML Post         │
    ├────────────────────┼───────────────────┤
    │ posts/yaml_post.md ┆ YAML Frontmatter  │
    └────────────────────┴───────────────────┘
    ```

=== "JSON"

    ```json
    [
      {
        "file": "posts/toml_post.md",
        "key": "title",
        "value": "TOML Post"
      },
      {
        "file": "posts/yaml_post.md",
        "key": "title",
        "value": "YAML Frontmatter"
      }
    ]
    ```

=== "Plain"

    ```text
    # Multiple files or directory input: <file>\t<value>
    posts/toml_post.md	TOML Post
    posts/yaml_post.md	YAML Frontmatter

    # Single explicit file with `get`: <value> only
    TOML Post
    ```

## :file_folder: File Discovery & Ignore Rules

When given directories (or no paths, defaulting to `.`), `mdfm` walks recursively for `.md` and `.markdown` files:

- Respects `.gitignore` rules (even outside an active Git repository).
- Applies any additional `ignore` glob patterns configured in `.mdfm.toml` or global config (for example, `ignore = ["drafts/**", "CHANGELOG.md"]`).
- Explicitly passed file paths are always processed directly.
