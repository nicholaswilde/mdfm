# Security

`mdfm` is designed to operate safely across large Markdown repositories and automated CI pipelines without corrupting document content, executing untrusted code, or making unexpected network calls.

## :shield: Data Integrity & Preservation Guarantees

- **Zero-Copy Body Preservation:** `mdfm` splits Markdown files on raw byte offsets and slices the body verbatim. Only the frontmatter slice between opening and closing delimiters is parsed or re-serialized; Markdown bodies (including mixed line endings, trailing whitespace, or code blocks) are never rewritten.
- **Format & Metadata Preservation:** UTF-8 BOMs (`\u{FEFF}`), `LF`/`CRLF` line endings, original YAML closing delimiters (`---` vs `...`), TOML delimiters (`+++`), and trailing-newline presence are preserved on round-trip.
- **Lossless TOML Editing:** TOML frontmatter is mutated through `toml_edit`, retaining inline comments, blank lines, formatting, and key order.
- **Unclosed Delimiter Safety:** An unclosed opening `---` (such as a Markdown horizontal rule) is treated as a body-only file rather than truncated or misparsed as frontmatter.
- **No-Op Write Avoidance:** `mdfm set` and `mdfm del` compare the rendered output against the original content and only write to disk when bytes actually change.

## :test_tube: Safe Bulk Mutation & Validation Practices

- **Preview Before Writing:** Always run bulk mutations with `--dry-run` first to verify which files would be created, updated, or modified:

    ```bash
    mdfm --where 'draft == true' set draft=false posts/ --dry-run
    mdfm del legacy_field posts/ --dry-run
    ```

- **Restrict Scope Explicitly:** Combine `--where` filters with project-local `ignore` rules in `.mdfm.toml` to keep generated files, fixtures, or drafts out of bulk operations.

- **Offline Schema Validation:** `mdfm check` compiles [`jsonschema`](https://github.com/Stranger6667/jsonschema) with `default-features = false`, performing pure-Rust validation locally without making outbound HTTP requests to resolve remote `$ref` schemas.

## :package: Supply Chain & Binary Verification

- **Memory-Safe Rust Binary:** Built as a static Rust CLI binary with zero `unsafe` blocks in `mdfm` source code.
- **Checksum Verification:** Every GitHub Release publishes `.sha256` checksum files alongside `.tar.gz` and `.zip` archives. Verify downloads before execution:

```bash
# Linux
sha256sum -c mdfm-<version>-<target>.tar.gz.sha256

# macOS
shasum -a 256 -c mdfm-<version>-<target>.tar.gz.sha256
```

## :bug: Reporting a Vulnerability

If you discover a security vulnerability (such as a parser crash on untrusted frontmatter, path-handling bug, or unintended file modification):

- **Private Disclosure:** Report privately via [GitHub Security Advisories](https://github.com/nicholaswilde/mdfm/security/advisories/new) so the issue can be assessed and patched before public disclosure.
- **Non-Sensitive Hardening:** General safety improvements or non-exploitable bugs can be filed as standard [GitHub issues](https://github.com/nicholaswilde/mdfm/issues).
