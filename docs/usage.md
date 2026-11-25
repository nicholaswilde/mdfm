# Usage

```bash
mdfm [OPTIONS] <COMMAND>
```

## :keyboard: Commands

| Command | Description | Example |
| :--- | :--- | :--- |
| `get` | Query a frontmatter key (supports dot paths) across Markdown files | `mdfm get author.name docs/` |
| `set` | Update or insert frontmatter keys (creates frontmatter if absent) | `mdfm set draft=false posts/ --dry-run --diff` |
| `del` | Delete one or more keys from frontmatter | `mdfm del draft author.bio posts/ --dry-run --diff` |
| `mv` (`rename`) | Atomically rename or merge frontmatter keys across files | `mdfm mv --merge tag tags docs/` |
| `check` | Validate frontmatter syntax and optionally against a JSON Schema | `mdfm check --schema schema.json posts/` |
| `keys` | Audit frontmatter keys, file counts, and observed types across files | `mdfm keys docs/` |
| `values` | Audit unique values and occurrence counts for a key across files | `mdfm values tags docs/` |
| `fmt` | Format frontmatter (YAML sequence indent, key/array sorting, YAML/TOML conversion) | `mdfm fmt --sort-keys --sort-array tags docs/` |
| `completions` | Generate shell completion script (`bash`, `zsh`, `fish`, `powershell`, `elvish`) | `mdfm completions zsh` |
| `man` | Output UNIX roff man page | `mdfm man > assets/mdfm.1` |
| `update` | Check for and install the latest `mdfm` release from GitHub | `mdfm update [--check] [-y] [--version <ver>]` |

## :gear: Global Options

Global flags may be placed before or after the subcommand:

| Flag | Environment Variable | Default | Description |
| :--- | :--- | :--- | :--- |
| `--format <yaml\|toml>` | `MDFM_FORMAT` | `yaml` | Frontmatter format used when creating new frontmatter on files without any |
| `--output <table\|json\|plain\|files>` | `MDFM_OUTPUT` | `table` | Output presentation format |
| `-l`, `--files-only` | N/A | `false` | Print only matching file paths, one per line (shorthand for `--output files`) |
| `--where <EXPR>` | N/A | None | Filter files by evaluating a frontmatter predicate expression |
| `-C`, `--config <PATH>` | `MDFM_CONFIG` | Auto-discover | Explicit configuration file path (`.toml`, `.yaml`, `.yml`, or `.json`) |
| `--ext <EXT>` | N/A | `md,markdown` | File extensions to discover during directory walks (repeatable or comma-separated) |
| `-h`, `--help` | N/A | N/A | Print command help |
| `-V`, `--version` | N/A | N/A | Print `mdfm` version |

## :mag: Querying Keys (`get`)

Query one or more top-level/nested keys — or dump the entire frontmatter object with `.` — across a single file, multiple paths, or a directory (defaults to `.` when no paths are passed):

```bash
# Query a single top-level or nested key across a directory
mdfm get title docs/
mdfm get author.name posts/

# Query multiple keys in a single pass (comma-separated or repeatable -k/--key)
mdfm get title,tags,author.name docs/
mdfm get -k title -k tags docs/

# Dump the complete frontmatter object per file with '.'
mdfm --output json get . posts/ | jq '.[].value'
mdfm --output plain get . posts/hello.md | jq .title

# Query a single file in plain format (emits only the value)
mdfm --output plain get title posts/hello.md

# Print only matching file paths (pipeable to xargs / rumdl)
mdfm --where 'draft == true' -l get title posts/
```

Files without frontmatter are skipped (unless matching a `has_frontmatter` / `frontmatter_format` `--where` predicate with `.`). In multi-key queries, files missing all requested keys are skipped; missing individual fields render as `null` in `json` and blank in `table` / `plain`.

## :pencil2: Updating & Inserting Keys (`set`)

Pass one or more `KEY=VALUE`, `KEY+=VALUE`, or `KEY-=VALUE` mutations followed by optional file or directory paths (defaults to `.`):

```bash
# Preview changes as a unified diff without modifying files on disk
mdfm set draft=false 'tags=["rust","cli"]' posts/ --dry-run --diff

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
# Preview deletions as a unified diff without modifying files
mdfm del draft legacy_id posts/ --dry-run --diff

# Delete top-level and nested keys
mdfm del draft author.bio posts/

# Delete keys only from files matching a --where filter
mdfm --where 'draft == false' del reviewer_notes posts/
```

The first positional argument is always parsed as a key path. Subsequent arguments remain keys until an argument exists on disk or resembles a filesystem path (`.`, `..`, contains `/` or `\`, or matches a configured extension such as `.md`/`.markdown`). Files without frontmatter are skipped.

## :twisted_rightwards_arrows: Renaming & Merging Keys (`mv` / `rename`)

Atomically rename or merge a frontmatter key (`<OLD_KEY>` -> `<NEW_KEY>`) across YAML and TOML files (supporting nested dot paths, `--dry-run`, `--diff`, and `--where`):

```bash
# Rename a nested key across a directory (preview unified diff first)
mdfm mv author.full_name author.name docs/ --dry-run --diff
mdfm rename author.full_name author.name docs/

# Merge typo key `tag` into `tags` (deduplicating arrays) and delete `tag`
mdfm mv --merge tag tags docs/

# Replace an existing destination key unconditionally
mdfm mv --overwrite legacy_title title docs/
```

- **Conflict safety:** If `<NEW_KEY>` already exists, `mv` exits with an error unless `--merge` or `--overwrite` is passed.
- **`--merge` rules:** Deduplicates union of arrays (including scalar-into-array merges), shallow-merges mappings (preserving existing destination subkeys), and errors on incompatible/conflicting scalar types.
- **Unified diff (`--diff`):** Available on `set`, `del`, and `mv`. In `table` and `plain` modes, prints unified diffs for changed files only (zero output for unchanged files); in `json` mode, attaches a `"diff"` string to changed entries.

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
| Document metadata | `has_frontmatter`, `frontmatter_format` | `not has_frontmatter`, `has_frontmatter == false`, `frontmatter_format == "toml"` |
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

# Find files lacking frontmatter completely (distinguished from empty --- blocks)
mdfm --where 'not has_frontmatter' -l get . docs/

# Add frontmatter only to files that have none
mdfm --where 'has_frontmatter == false' set comments=true docs/

# List only TOML frontmatter files
mdfm --where 'frontmatter_format == "toml"' -l get . posts/

# Validate only non-draft posts against a strict publishing schema
mdfm --where 'not draft' check --schema publish-schema.json posts/
```

!!! tip
    Bare key paths evaluate as falsy when missing, `null`, `false`, `0`, `""`, `[]`, or `{}`. Inequality (`!=`) evaluates to `true` when the referenced key is absent. Use `has_frontmatter` or `frontmatter_format` (`"yaml"`, `"toml"`, or `null`) to target files without frontmatter; quote `"has_frontmatter"` if querying a literal frontmatter key with that name.

## :white_check_mark: Schema & Syntax Validation (`check`)

Validate YAML and TOML frontmatter syntax and, when configured, validate against any standard JSON Schema file (such as Draft 2020-12). `mdfm check` exits with a non-zero status code when any file fails parsing or validation, making it ready for CI pipelines and pre-commit hooks.

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
# Syntax-only validation (automatic when no schema is configured)
mdfm check docs/

# Explicitly bypass any configured .mdfm.toml schema / [[schemas]] rules
mdfm check --syntax-only cook/

# Validate a directory with an explicit schema
mdfm check --schema .mdfm-schema.json docs/

# CI-friendly quiet mode: hide passing files and emit a compact summary
mdfm check --errors-only --schema .mdfm-schema.json docs/
mdfm check -q docs/

# Emit machine-readable validation failures only as JSON
mdfm --output json check --errors-only --schema .mdfm-schema.json docs/

# Rely on `schema = ".mdfm-schema.json"` or `[[schemas]]` in .mdfm.toml or MDFM_SCHEMA
mdfm check docs/
```

When no `--schema`, `MDFM_SCHEMA`, `schema`, or `[[schemas]]` is configured — or when `--syntax-only` is passed — `mdfm check` validates YAML/TOML frontmatter parseability without requiring a JSON Schema. Body-only files pass syntax-only checks, while malformed YAML/TOML blocks (such as unquoted emoji shortcodes like `title: :cake: Pumpkin Cake`) are reported as validation failures.

When `--errors-only` (or `-q` / `--quiet`, or `errors_only = true` in `.mdfm.toml`) is enabled:

- **`table` / `plain`:** Passing files are omitted and a compact summary line (`<ok> ok, <failed> failed`) is printed. When all files pass, only the summary line is emitted.
- **`json`:** Only failing entries are included in the output JSON array (`[]` when all files pass).

When a JSON Schema is active, files without frontmatter are validated as an empty JSON object (`{}`) unless filtered out by `--where`. Malformed YAML or TOML frontmatter is reported as a validation failure for that file.

## :clipboard: Taxonomy Audits (`keys` & `values`)

Inspect frontmatter key usage, type drift, and value frequencies across a repository without external scripts:

```bash
# List every frontmatter key (including nested dot-paths), file count, and observed JSON types
mdfm keys docs/
mdfm keys --sort alpha docs/
mdfm --output json keys docs/

# List unique values and occurrence counts for a key (flattens arrays by default)
mdfm values tags docs/
mdfm values tags --sort alpha docs/

# Keep array values intact instead of flattening elements
mdfm values tags --no-flatten docs/

# Audit only matching files with --where
mdfm --where 'draft == false' values author.role docs/
```

- **Key audit (`keys`):** Walks nested objects recursively (reporting both parent objects and dot-path leaf keys) and aggregates observed JSON types (`string`, `number`, `boolean`, `array`, `object`, `null`) to highlight typos (e.g. `tag` vs `tags`) and type drift (e.g. `array, string`).
- **Value audit (`values <KEY>`):** Flattens array values into individual element counts by default (`--no-flatten` counts whole arrays), supports nested dot paths, `--where` filtering, `--sort <count|alpha>` (default: `count` descending), and `--output <table|json|plain|files>`.

## :sparkles: Formatting Frontmatter (`fmt`)

Normalize frontmatter style (YAML sequence indentation, recursive key sorting, array sorting/deduplication, and YAML <-> TOML conversion) without modifying Markdown bodies, UTF-8 BOM, or line endings:

```bash
# Format frontmatter in-place (defaults to 2-space indented YAML sequences)
mdfm fmt docs/

# CI check mode: exit non-zero if any file needs formatting (combine with --diff for preview)
mdfm fmt --check docs/
mdfm fmt --check --diff docs/

# Sort keys alphabetically and sort + deduplicate specific arrays
mdfm fmt --sort-keys --sort-array tags docs/
mdfm fmt --sort-array tags,categories --dry-run --diff docs/

# Customize YAML indentation width or disable sequence indentation
mdfm fmt --yaml-indent 4 docs/
mdfm fmt --indent-sequences false docs/

# Convert frontmatter between YAML (---) and TOML (+++)
mdfm fmt --to toml docs/
mdfm fmt --to yaml docs/
```

- **Shared YAML indentation:** `indent_sequences` (default: `true`) and `yaml_indent` (default: `2`) configured in `.mdfm.toml` are shared across `fmt`, `set`, `del`, and `mv`, keeping `yamllint` / Prettier 2-space sequence indentation (`tags:\n  - item`) intact on every edit.
- **Safety:** Body-only files are skipped. Converting YAML containing `null` values to TOML fails cleanly with an error rather than dropping data.

## :bar_chart: Output Formats

All subcommands support `--output table` (default), `--output json`, `--output plain`, and `--output files` (`-l` / `--files-only`):

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

=== "Files (`-l` / `--files-only`)"

    ```text
    posts/toml_post.md
    posts/yaml_post.md
    ```

## :file_folder: File Discovery & Ignore Rules

When given directories (or no paths, defaulting to `.`), `mdfm` walks recursively for files matching the configured extensions (default: `.md` and `.markdown`):

- Customize discovered extensions via repeatable or comma-separated `--ext` flags (e.g. `mdfm --ext cook --ext mdx get title .` or `mdfm --ext cook,mdx check .`) or `extensions = ["md", "cook"]` in `.mdfm.toml`. Extension matching is case-insensitive and strips any leading `.`.
- Respects `.gitignore` rules (even outside an active Git repository).
- Applies any additional `ignore` glob patterns configured in `.mdfm.toml` or global config (for example, `ignore = ["drafts/**", "CHANGELOG.md"]`).
- Explicitly passed file paths are always processed directly regardless of extension.
