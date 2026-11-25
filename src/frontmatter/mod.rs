pub mod filter;
pub mod toml;
pub mod yaml;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

/// Supported frontmatter serialization formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Yaml,
    Toml,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yaml => f.write_str("yaml"),
            Self::Toml => f.write_str("toml"),
        }
    }
}

/// Detected line ending style of a Markdown file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
        }
    }
}

/// Closing delimiter marker preserved from the original frontmatter block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosingDelimiter {
    Dashes,
    Dots,
    Pluses,
}

impl ClosingDelimiter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dashes => "---",
            Self::Dots => "...",
            Self::Pluses => "+++",
        }
    }

    pub fn default_for(format: Format) -> Self {
        match format {
            Format::Yaml => Self::Dashes,
            Format::Toml => Self::Pluses,
        }
    }
}

/// Errors returned by frontmatter parsing, manipulation, and rendering.
#[derive(Debug, Error)]
pub enum FrontmatterError {
    #[error("invalid YAML frontmatter: {0}")]
    YamlParse(#[from] serde_yaml_ng::Error),

    #[error("invalid TOML frontmatter: {0}")]
    TomlParse(#[from] toml_edit::TomlError),

    #[error("frontmatter root must be a key-value mapping ({format})")]
    InvalidRoot { format: Format },

    #[error("invalid key path: '{0}'")]
    InvalidKeyPath(String),

    #[error("cannot traverse non-mapping key '{key}' in path '{path}'")]
    NotAMapping { key: String, path: String },

    #[error("unsupported value for {format} frontmatter: {reason}")]
    UnsupportedValue { format: Format, reason: String },
}

/// Zero-copy slice of a frontmatter block extracted from a Markdown document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterBlock<'a> {
    pub format: Format,
    /// Raw frontmatter content between opening and closing delimiter lines.
    pub raw: &'a str,
    /// Original closing delimiter (`---`, `...`, or `+++`).
    pub close_delim: ClosingDelimiter,
    /// Whether a line ending immediately followed the closing delimiter line.
    pub trailing_newline: bool,
}

/// Result of splitting a Markdown document into frontmatter metadata and verbatim body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitDocument<'a> {
    pub has_bom: bool,
    pub line_ending: LineEnding,
    pub frontmatter: Option<FrontmatterBlock<'a>>,
    /// Untouched verbatim slice of the Markdown body (or full non-BOM document if no frontmatter).
    pub body: &'a str,
}

/// Unified frontmatter data model supporting YAML mappings and lossless TOML documents.
#[derive(Debug, Clone)]
pub enum FrontmatterData {
    Yaml(serde_yaml_ng::Mapping),
    Toml(toml_edit::DocumentMut),
}

impl FrontmatterData {
    /// Create an empty frontmatter mapping/document in the specified format.
    pub fn empty(format: Format) -> Self {
        match format {
            Format::Yaml => Self::Yaml(serde_yaml_ng::Mapping::new()),
            Format::Toml => Self::Toml(toml_edit::DocumentMut::new()),
        }
    }

    /// Parse raw frontmatter text according to `format`.
    pub fn parse(format: Format, raw: &str) -> Result<Self, FrontmatterError> {
        match format {
            Format::Yaml => Ok(Self::Yaml(yaml::parse_yaml(raw)?)),
            Format::Toml => Ok(Self::Toml(toml::parse_toml(raw)?)),
        }
    }

    /// Return the underlying frontmatter format.
    pub fn format(&self) -> Format {
        match self {
            Self::Yaml(_) => Format::Yaml,
            Self::Toml(_) => Format::Toml,
        }
    }

    /// Query a value by dot-separated key path (e.g. `author.name`).
    pub fn get(&self, path: &str) -> Option<JsonValue> {
        match self {
            Self::Yaml(mapping) => yaml::get(mapping, path),
            Self::Toml(doc) => toml::get(doc, path),
        }
    }

    /// Insert or update a value at a dot-separated key path.
    pub fn set(&mut self, path: &str, value: JsonValue) -> Result<(), FrontmatterError> {
        match self {
            Self::Yaml(mapping) => yaml::set(mapping, path, value),
            Self::Toml(doc) => toml::set(doc, path, value),
        }
    }

    /// Delete a key at a dot-separated path. Returns `true` if a key was removed.
    pub fn del(&mut self, path: &str) -> Result<bool, FrontmatterError> {
        match self {
            Self::Yaml(mapping) => yaml::del(mapping, path),
            Self::Toml(doc) => toml::del(doc, path),
        }
    }

    /// Convert the entire frontmatter mapping into a `serde_json::Value::Object`.
    pub fn to_json_value(&self) -> Result<JsonValue, FrontmatterError> {
        match self {
            Self::Yaml(mapping) => yaml::mapping_to_json(mapping),
            Self::Toml(doc) => toml::document_to_json(doc),
        }
    }

    /// Serialize frontmatter back to its native text representation (without delimiters).
    pub fn serialize(&self) -> Result<String, FrontmatterError> {
        match self {
            Self::Yaml(mapping) => yaml::serialize_yaml(mapping),
            Self::Toml(doc) => Ok(toml::serialize_toml(doc)),
        }
    }
}

/// Split a dot-separated key path into validated non-empty segments.
pub(crate) fn split_key_path(path: &str) -> Result<Vec<&str>, FrontmatterError> {
    if path.is_empty() {
        return Err(FrontmatterError::InvalidKeyPath(path.to_string()));
    }
    let segments: Vec<&str> = path.split('.').collect();
    if segments.iter().any(|s| s.is_empty()) {
        return Err(FrontmatterError::InvalidKeyPath(path.to_string()));
    }
    Ok(segments)
}

/// Read the next line starting at byte offset `start` without using `str::lines()`.
///
/// Returns `(line_without_eol, eol_start, next_line_start)`.
fn next_line(s: &str, start: usize) -> Option<(&str, usize, usize)> {
    if start >= s.len() {
        return None;
    }
    let bytes = s.as_bytes();
    match bytes[start..].iter().position(|&b| b == b'\n') {
        Some(rel_nl) => {
            let nl = start + rel_nl;
            let eol_start = if nl > start && bytes[nl - 1] == b'\r' {
                nl - 1
            } else {
                nl
            };
            Some((&s[start..eol_start], eol_start, nl + 1))
        }
        None => Some((&s[start..], s.len(), s.len())),
    }
}

/// Detect the line ending style from the first `\n` byte in `s`.
fn detect_line_ending(s: &str) -> LineEnding {
    let bytes = s.as_bytes();
    if let Some(nl) = bytes.iter().position(|&b| b == b'\n') {
        if nl > 0 && bytes[nl - 1] == b'\r' {
            return LineEnding::Crlf;
        }
    }
    LineEnding::Lf
}

/// Split a Markdown document into optional frontmatter and verbatim body using byte offsets.
pub fn split(input: &str) -> SplitDocument<'_> {
    let (has_bom, rest) = match input.strip_prefix('\u{FEFF}') {
        Some(stripped) => (true, stripped),
        None => (false, input),
    };

    let line_ending = detect_line_ending(rest);

    let Some((first_line, eol_start, fm_start)) = next_line(rest, 0) else {
        return SplitDocument {
            has_bom,
            line_ending,
            frontmatter: None,
            body: rest,
        };
    };

    // An opening delimiter must be followed by a line break so a closing delimiter can exist.
    if fm_start == eol_start {
        return SplitDocument {
            has_bom,
            line_ending,
            frontmatter: None,
            body: rest,
        };
    }

    let trimmed_open = first_line.trim_end_matches([' ', '\t']);
    let format = match trimmed_open {
        "---" => Format::Yaml,
        "+++" => Format::Toml,
        _ => {
            return SplitDocument {
                has_bom,
                line_ending,
                frontmatter: None,
                body: rest,
            };
        }
    };

    let mut pos = fm_start;
    while let Some((line, line_eol_start, next_pos)) = next_line(rest, pos) {
        let trimmed = line.trim_end_matches([' ', '\t']);
        let close_delim = match format {
            Format::Yaml if trimmed == "---" => Some(ClosingDelimiter::Dashes),
            Format::Yaml if trimmed == "..." => Some(ClosingDelimiter::Dots),
            Format::Toml if trimmed == "+++" => Some(ClosingDelimiter::Pluses),
            _ => None,
        };

        if let Some(close_delim) = close_delim {
            let raw = &rest[fm_start..pos];
            let trailing_newline = next_pos > line_eol_start;
            let body = &rest[next_pos..];
            return SplitDocument {
                has_bom,
                line_ending,
                frontmatter: Some(FrontmatterBlock {
                    format,
                    raw,
                    close_delim,
                    trailing_newline,
                }),
                body,
            };
        }

        pos = next_pos;
    }

    // Unclosed opening delimiter (e.g. a Markdown horizontal rule) -> body-only.
    SplitDocument {
        has_bom,
        line_ending,
        frontmatter: None,
        body: rest,
    }
}

/// Normalize serialized frontmatter text to `line_ending`, guaranteeing a single trailing EOL
/// whenever the frontmatter payload is non-empty.
fn normalize_frontmatter_lines(serialized: &str, line_ending: LineEnding) -> String {
    if serialized.is_empty() {
        return String::new();
    }

    let mut lf = serialized.replace("\r\n", "\n");
    if !lf.ends_with('\n') {
        lf.push('\n');
    }

    match line_ending {
        LineEnding::Lf => lf,
        LineEnding::Crlf => lf.replace('\n', "\r\n"),
    }
}

impl<'a> SplitDocument<'a> {
    /// Parse the extracted frontmatter block (if present) into a `FrontmatterData`.
    pub fn parse_frontmatter(&self) -> Result<Option<FrontmatterData>, FrontmatterError> {
        match &self.frontmatter {
            Some(block) => FrontmatterData::parse(block.format, block.raw).map(Some),
            None => Ok(None),
        }
    }

    /// Render the document back to a `String`, preserving UTF-8 BOM, line endings,
    /// closing delimiter style, trailing newline presence, and the verbatim `body` slice.
    pub fn render(&self, data: Option<&FrontmatterData>) -> Result<String, FrontmatterError> {
        let mut out = String::with_capacity(self.body.len() + 128);

        if self.has_bom {
            out.push('\u{FEFF}');
        }

        if let Some(fm) = data {
            let eol = self.line_ending.as_str();
            let open_delim = match fm.format() {
                Format::Yaml => "---",
                Format::Toml => "+++",
            };

            let (close_delim, trailing_newline) = match &self.frontmatter {
                Some(block) if block.format == fm.format() => {
                    (block.close_delim.as_str(), block.trailing_newline)
                }
                Some(block) => (
                    ClosingDelimiter::default_for(fm.format()).as_str(),
                    block.trailing_newline,
                ),
                None => (ClosingDelimiter::default_for(fm.format()).as_str(), true),
            };

            let serialized = fm.serialize()?;
            let normalized = normalize_frontmatter_lines(&serialized, self.line_ending);

            out.push_str(open_delim);
            out.push_str(eol);
            out.push_str(&normalized);
            out.push_str(close_delim);
            if trailing_newline {
                out.push_str(eol);
            }
        }

        out.push_str(self.body);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn yaml_dashes_round_trip() {
        let input = "---\ntitle: Hello World\ndraft: true\n---\n\n# Heading\n\nBody text.\n";
        let doc = split(input);

        assert!(!doc.has_bom);
        assert_eq!(doc.line_ending, LineEnding::Lf);
        let block = doc.frontmatter.as_ref().expect("expected frontmatter");
        assert_eq!(block.format, Format::Yaml);
        assert_eq!(block.close_delim, ClosingDelimiter::Dashes);
        assert!(block.trailing_newline);
        assert_eq!(doc.body, "\n# Heading\n\nBody text.\n");

        let mut fm = doc.parse_frontmatter().unwrap().unwrap();
        assert_eq!(fm.get("title"), Some(json!("Hello World")));
        assert_eq!(fm.get("draft"), Some(json!(true)));

        fm.set("draft", json!(false)).unwrap();
        let rendered = doc.render(Some(&fm)).unwrap();
        assert!(rendered.starts_with("---\n"));
        assert!(rendered.contains("draft: false\n---\n"));
        assert!(rendered.ends_with("\n# Heading\n\nBody text.\n"));
    }

    #[test]
    fn yaml_dots_closing_delimiter_round_trip() {
        let input = "---  \ntitle: Dots\n...\t\nBody line 1\nBody line 2";
        let doc = split(input);

        let block = doc.frontmatter.as_ref().expect("expected frontmatter");
        assert_eq!(block.format, Format::Yaml);
        assert_eq!(block.close_delim, ClosingDelimiter::Dots);
        assert!(block.trailing_newline);
        assert_eq!(doc.body, "Body line 1\nBody line 2");

        let fm = doc.parse_frontmatter().unwrap().unwrap();
        let rendered = doc.render(Some(&fm)).unwrap();
        assert_eq!(rendered, "---\ntitle: Dots\n...\nBody line 1\nBody line 2");
    }

    #[test]
    fn toml_pluses_round_trip() {
        let input = "+++\ntitle = \"TOML Post\"\ndraft = false\n+++\n\nBody content.\n";
        let doc = split(input);

        let block = doc.frontmatter.as_ref().expect("expected frontmatter");
        assert_eq!(block.format, Format::Toml);
        assert_eq!(block.close_delim, ClosingDelimiter::Pluses);
        assert!(block.trailing_newline);
        assert_eq!(doc.body, "\nBody content.\n");

        let fm = doc.parse_frontmatter().unwrap().unwrap();
        assert_eq!(fm.get("title"), Some(json!("TOML Post")));
        assert_eq!(fm.get("draft"), Some(json!(false)));

        let rendered = doc.render(Some(&fm)).unwrap();
        assert_eq!(rendered, input);
    }

    #[test]
    fn crlf_preservation_without_bare_lf() {
        let input = "---\r\ntitle: Windows\r\ndraft: true\r\n---\r\n\r\nLine 1\r\nLine 2\r\n";
        let doc = split(input);
        assert_eq!(doc.line_ending, LineEnding::Crlf);
        assert_eq!(doc.body, "\r\nLine 1\r\nLine 2\r\n");

        let mut fm = doc.parse_frontmatter().unwrap().unwrap();
        fm.set("author.name", json!("Nick")).unwrap();
        let rendered = doc.render(Some(&fm)).unwrap();

        // Ensure zero bare `\n` exist anywhere in output.
        let bytes = rendered.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                assert!(
                    i > 0 && bytes[i - 1] == b'\r',
                    "found bare LF at byte index {i} in {rendered:?}"
                );
            }
        }
        assert!(rendered.ends_with("\r\nLine 1\r\nLine 2\r\n"));
    }

    #[test]
    fn utf8_bom_preservation() {
        let input = "\u{FEFF}---\ntitle: BOM Test\n---\nBody\n";
        let doc = split(input);
        assert!(doc.has_bom);
        assert_eq!(doc.body, "Body\n");

        let mut fm = doc.parse_frontmatter().unwrap().unwrap();
        fm.set("reviewed", json!(true)).unwrap();
        let rendered = doc.render(Some(&fm)).unwrap();

        assert!(rendered.starts_with("\u{FEFF}---\n"));
        assert!(rendered.ends_with("---\nBody\n"));

        // Body-only file with BOM also preserves BOM verbatim.
        let body_only = "\u{FEFF}# Just Markdown\n";
        let doc2 = split(body_only);
        assert!(doc2.has_bom);
        assert!(doc2.frontmatter.is_none());
        assert_eq!(doc2.render(None).unwrap(), body_only);
    }

    #[test]
    fn empty_frontmatter_blocks() {
        let yaml_empty = "---\n---\nBody\n";
        let doc = split(yaml_empty);
        let fm = doc.parse_frontmatter().unwrap().unwrap();
        assert_eq!(fm.to_json_value().unwrap(), json!({}));
        assert_eq!(doc.render(Some(&fm)).unwrap(), yaml_empty);

        let yaml_no_trailing_nl = "---\n---";
        let doc2 = split(yaml_no_trailing_nl);
        let block2 = doc2.frontmatter.as_ref().unwrap();
        assert!(!block2.trailing_newline);
        let fm2 = doc2.parse_frontmatter().unwrap().unwrap();
        assert_eq!(doc2.render(Some(&fm2)).unwrap(), yaml_no_trailing_nl);

        let toml_empty = "+++\r\n+++\r\nBody\r\n";
        let doc3 = split(toml_empty);
        let fm3 = doc3.parse_frontmatter().unwrap().unwrap();
        assert_eq!(fm3.to_json_value().unwrap(), json!({}));
        assert_eq!(doc3.render(Some(&fm3)).unwrap(), toml_empty);
    }

    #[test]
    fn unclosed_horizontal_rule_is_body_only() {
        let input = "---\n\n# Heading after horizontal rule\n\nParagraph.\n";
        let doc = split(input);
        assert!(doc.frontmatter.is_none());
        assert_eq!(doc.body, input);
        assert_eq!(doc.render(None).unwrap(), input);

        // Single line "---" without trailing newline is also body-only.
        let single = "---";
        let doc_single = split(single);
        assert!(doc_single.frontmatter.is_none());
        assert_eq!(doc_single.body, single);
    }

    #[test]
    fn lossless_toml_comment_and_order_preservation() {
        let input = "\
+++
# Primary post metadata
z_last = \"keep first\" # inline comment
a_first = 1

# Author block
[author]
name = \"Old Name\" # keep this comment
role = \"writer\"
+++

# Markdown Body
";
        let doc = split(input);
        let mut fm = doc.parse_frontmatter().unwrap().unwrap();

        fm.set("author.name", json!("Nick")).unwrap();
        fm.set("draft", json!(false)).unwrap();

        let rendered = doc.render(Some(&fm)).unwrap();
        assert!(rendered.contains("# Primary post metadata\n"));
        assert!(rendered.contains("z_last = \"keep first\" # inline comment\n"));
        assert!(rendered.contains("# Author block\n"));
        assert!(rendered.contains("name = \"Nick\" # keep this comment\n"));
        assert!(rendered.contains("role = \"writer\"\n"));
        assert!(rendered.contains("draft = false\n"));

        // Verify original ordering (z_last still precedes a_first).
        let z_pos = rendered.find("z_last").unwrap();
        let a_pos = rendered.find("a_first").unwrap();
        assert!(z_pos < a_pos, "TOML key order was not preserved");

        // Body preserved verbatim.
        assert!(rendered.ends_with("\n# Markdown Body\n"));
    }

    #[test]
    fn create_frontmatter_on_body_only_file_preserves_body_and_eol() {
        let input = "# Plain Note\r\n\r\nNo frontmatter yet.\r\n";
        let doc = split(input);
        assert!(doc.frontmatter.is_none());
        assert_eq!(doc.line_ending, LineEnding::Crlf);

        let mut fm = FrontmatterData::empty(Format::Toml);
        fm.set("title", json!("Plain Note")).unwrap();
        let rendered = doc.render(Some(&fm)).unwrap();

        assert_eq!(
            rendered,
            "+++\r\ntitle = \"Plain Note\"\r\n+++\r\n# Plain Note\r\n\r\nNo frontmatter yet.\r\n"
        );
    }

    #[test]
    fn body_mixed_line_endings_untouched_verbatim() {
        let body = "Line LF\nLine CRLF\r\nTrailing no newline";
        let input = format!("---\ntitle: Test\n---\n{body}");
        let doc = split(&input);
        let mut fm = doc.parse_frontmatter().unwrap().unwrap();
        fm.set("title", json!("Updated")).unwrap();
        let rendered = doc.render(Some(&fm)).unwrap();
        assert!(rendered.ends_with(body));
    }

    #[test]
    fn empty_input_invalid_key_paths_and_format_switch_render() {
        // Empty input yields body-only with LF line ending.
        let empty = split("");
        assert!(!empty.has_bom);
        assert_eq!(empty.line_ending, LineEnding::Lf);
        assert!(empty.frontmatter.is_none());
        assert_eq!(empty.body, "");

        // Display & empty/del paths for both formats.
        assert_eq!(Format::Yaml.to_string(), "yaml");
        assert_eq!(Format::Toml.to_string(), "toml");
        let mut y = FrontmatterData::empty(Format::Yaml);
        y.set("a", json!(1)).unwrap();
        let mut t = FrontmatterData::empty(Format::Toml);
        t.set("a", json!(1)).unwrap();
        assert!(t.del("a").unwrap());
        assert!(!t.del("a").unwrap());

        // Invalid key paths.
        assert!(matches!(
            split_key_path(""),
            Err(FrontmatterError::InvalidKeyPath(_))
        ));
        assert!(matches!(
            split_key_path("a..b"),
            Err(FrontmatterError::InvalidKeyPath(_))
        ));

        // Rendering a TOML FrontmatterData into a SplitDocument whose original block was YAML.
        let doc = split("---\ntitle: Old\n---\nBody\n");
        let mut new_toml = FrontmatterData::empty(Format::Toml);
        new_toml.set("title", json!("New")).unwrap();
        let rendered = doc.render(Some(&new_toml)).unwrap();
        assert_eq!(rendered, "+++\ntitle = \"New\"\n+++\nBody\n");

        // Rendering YAML FrontmatterData into a body-only document uses default `---` delimiter.
        let plain = split("# Hello\n");
        let rendered_yaml = plain.render(Some(&y)).unwrap();
        assert_eq!(rendered_yaml, "---\na: 1\n---\n# Hello\n");

        // normalize_frontmatter_lines appends EOL when serialized text lacks trailing newline.
        assert_eq!(
            normalize_frontmatter_lines("a = 1", LineEnding::Lf),
            "a = 1\n"
        );
    }
}
