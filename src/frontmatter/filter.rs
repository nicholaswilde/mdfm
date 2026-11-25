use crate::frontmatter::Format;
use serde_json::Value as JsonValue;
use thiserror::Error;

/// Errors produced when parsing a `--where` filter expression.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FilterError {
    #[error("empty filter expression")]
    Empty,

    #[error("unexpected token '{token}' at position {pos}")]
    UnexpectedToken { token: String, pos: usize },

    #[error("unexpected end of filter expression")]
    UnexpectedEof,

    #[error("unterminated string literal starting at position {pos}")]
    UnterminatedString { pos: usize },

    #[error("unterminated quoted key starting at position {pos}")]
    UnterminatedQuotedKey { pos: usize },
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Ident(String),
    Backtick(String),
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
    And,
    Or,
    Not,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    pos: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
}

/// Document-level metadata fields queryable in `--where` expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaField {
    HasFrontmatter,
    FrontmatterFormat,
}

/// Document context passed to `FilterExpr::matches_doc`.
#[derive(Debug, Clone, Copy)]
pub struct FilterContext<'a> {
    /// Parsed frontmatter JSON object (`None` for files without frontmatter).
    pub frontmatter: Option<&'a JsonValue>,
    /// Detected frontmatter format (`None` for files without frontmatter).
    pub format: Option<Format>,
}

/// Parsed AST for `--where` filter expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Compare {
        path: Vec<String>,
        op: CompareOp,
        value: JsonValue,
    },
    Truthy(Vec<String>),
    MetaCompare {
        field: MetaField,
        op: CompareOp,
        value: JsonValue,
    },
    MetaTruthy(MetaField),
}

impl FilterExpr {
    /// Parse a filter expression such as `draft == true`, `author.name == "Nick"`,
    /// `not has_frontmatter`, `frontmatter_format == "toml"`,
    /// `` `prep time` != null ``, `meta["prep time"] == "5 minutes"`,
    /// or `tags contains "rust" and draft == false`.
    pub fn parse(input: &str) -> Result<Self, FilterError> {
        let tokens = tokenize(input)?;
        if tokens.is_empty() {
            return Err(FilterError::Empty);
        }
        let mut parser = Parser { tokens, idx: 0 };
        let expr = parser.parse_or()?;
        if let Some(tok) = parser.peek() {
            return Err(FilterError::UnexpectedToken {
                token: format!("{:?}", tok.kind),
                pos: tok.pos,
            });
        }
        Ok(expr)
    }

    /// Return `true` if the expression references document-level frontmatter metadata
    /// (`has_frontmatter` or `frontmatter_format`).
    pub fn references_document_meta(&self) -> bool {
        match self {
            Self::And(lhs, rhs) | Self::Or(lhs, rhs) => {
                lhs.references_document_meta() || rhs.references_document_meta()
            }
            Self::Not(inner) => inner.references_document_meta(),
            Self::MetaCompare { .. } | Self::MetaTruthy(_) => true,
            Self::Compare { .. } | Self::Truthy(_) => false,
        }
    }

    /// Evaluate the filter expression against a parsed frontmatter JSON object.
    pub fn matches(&self, root: &JsonValue) -> bool {
        self.matches_doc(FilterContext {
            frontmatter: Some(root),
            format: None,
        })
    }

    /// Evaluate the filter expression against a document context (supporting files without
    /// frontmatter when the expression references `has_frontmatter` or `frontmatter_format`).
    pub fn matches_doc(&self, ctx: FilterContext<'_>) -> bool {
        if ctx.frontmatter.is_none() && !self.references_document_meta() {
            return false;
        }
        self.eval_doc(ctx)
    }

    fn eval_doc(&self, ctx: FilterContext<'_>) -> bool {
        match self {
            Self::And(lhs, rhs) => lhs.eval_doc(ctx) && rhs.eval_doc(ctx),
            Self::Or(lhs, rhs) => lhs.eval_doc(ctx) || rhs.eval_doc(ctx),
            Self::Not(inner) => !inner.eval_doc(ctx),
            Self::Truthy(path) => ctx
                .frontmatter
                .and_then(|root| lookup_path(root, path))
                .is_some_and(is_truthy),
            Self::Compare { path, op, value } => {
                let actual = ctx.frontmatter.and_then(|root| lookup_path(root, path));
                evaluate_compare(actual, *op, value)
            }
            Self::MetaTruthy(field) => is_truthy(&meta_value(*field, ctx)),
            Self::MetaCompare { field, op, value } => {
                let actual = meta_value(*field, ctx);
                evaluate_compare(Some(&actual), *op, value)
            }
        }
    }
}

fn meta_value(field: MetaField, ctx: FilterContext<'_>) -> JsonValue {
    match field {
        MetaField::HasFrontmatter => JsonValue::Bool(ctx.frontmatter.is_some()),
        MetaField::FrontmatterFormat => match ctx.format {
            Some(fmt) => JsonValue::String(fmt.to_string()),
            None => JsonValue::Null,
        },
    }
}

fn is_truthy(val: &JsonValue) -> bool {
    match val {
        JsonValue::Null => false,
        JsonValue::Bool(b) => *b,
        JsonValue::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        JsonValue::String(s) => !s.is_empty(),
        JsonValue::Array(a) => !a.is_empty(),
        JsonValue::Object(o) => !o.is_empty(),
    }
}

fn lookup_path<'a>(root: &'a JsonValue, path: &[String]) -> Option<&'a JsonValue> {
    let mut current = root;
    for seg in path {
        match current {
            JsonValue::Object(map) => current = map.get(seg.as_str())?,
            JsonValue::Array(arr) => {
                let idx = seg.parse::<usize>().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn json_values_equal(a: &JsonValue, b: &JsonValue) -> bool {
    match (a, b) {
        (JsonValue::Number(na), JsonValue::Number(nb)) => {
            if let (Some(ia), Some(ib)) = (na.as_i64(), nb.as_i64()) {
                ia == ib
            } else if let (Some(fa), Some(fb)) = (na.as_f64(), nb.as_f64()) {
                (fa - fb).abs() < f64::EPSILON
            } else {
                na == nb
            }
        }
        _ => a == b,
    }
}

fn compare_order(a: &JsonValue, b: &JsonValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (JsonValue::Number(na), JsonValue::Number(nb)) => {
            let fa = na.as_f64()?;
            let fb = nb.as_f64()?;
            fa.partial_cmp(&fb)
        }
        (JsonValue::String(sa), JsonValue::String(sb)) => Some(sa.cmp(sb)),
        _ => None,
    }
}

fn evaluate_compare(actual: Option<&JsonValue>, op: CompareOp, expected: &JsonValue) -> bool {
    match op {
        CompareOp::Eq => actual.is_some_and(|a| json_values_equal(a, expected)),
        CompareOp::Ne => match actual {
            Some(a) => !json_values_equal(a, expected),
            None => true,
        },
        CompareOp::Gt => actual
            .and_then(|a| compare_order(a, expected))
            .is_some_and(|o| o.is_gt()),
        CompareOp::Ge => actual
            .and_then(|a| compare_order(a, expected))
            .is_some_and(|o| o.is_ge()),
        CompareOp::Lt => actual
            .and_then(|a| compare_order(a, expected))
            .is_some_and(|o| o.is_lt()),
        CompareOp::Le => actual
            .and_then(|a| compare_order(a, expected))
            .is_some_and(|o| o.is_le()),
        CompareOp::Contains => match actual {
            Some(JsonValue::Array(items)) => items.iter().any(|it| json_values_equal(it, expected)),
            Some(JsonValue::String(s)) => {
                expected.as_str().is_some_and(|needle| s.contains(needle))
            }
            _ => false,
        },
    }
}

fn scan_quoted(input: &str, start: usize, quote: char) -> Result<(String, usize), FilterError> {
    let unterminated = || {
        if quote == '`' {
            FilterError::UnterminatedQuotedKey { pos: start }
        } else {
            FilterError::UnterminatedString { pos: start }
        }
    };
    let after_open = start + quote.len_utf8();
    let mut buf = String::new();
    let mut chars = input[after_open..].char_indices();
    while let Some((rel, ch)) = chars.next() {
        if ch == '\\' {
            let Some((_, esc)) = chars.next() else {
                return Err(unterminated());
            };
            match esc {
                'n' => buf.push('\n'),
                'r' => buf.push('\r'),
                't' => buf.push('\t'),
                '\\' => buf.push('\\'),
                '"' => buf.push('"'),
                '\'' => buf.push('\''),
                '`' => buf.push('`'),
                other => {
                    buf.push('\\');
                    buf.push(other);
                }
            }
        } else if ch == quote {
            return Ok((buf, after_open + rel + ch.len_utf8()));
        } else {
            buf.push(ch);
        }
    }
    Err(unterminated())
}

fn tokenize(input: &str) -> Result<Vec<Token>, FilterError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        let start = i;
        let next = bytes.get(i + 1).copied();
        let punct = match (b, next) {
            (b'(', _) => Some((TokenKind::LParen, 1)),
            (b')', _) => Some((TokenKind::RParen, 1)),
            (b'[', _) => Some((TokenKind::LBracket, 1)),
            (b']', _) => Some((TokenKind::RBracket, 1)),
            (b'.', _) => Some((TokenKind::Dot, 1)),
            (b'=', Some(b'=')) => Some((TokenKind::Eq, 2)),
            (b'!', Some(b'=')) => Some((TokenKind::Ne, 2)),
            (b'!', _) => Some((TokenKind::Not, 1)),
            (b'>', Some(b'=')) => Some((TokenKind::Ge, 2)),
            (b'>', _) => Some((TokenKind::Gt, 1)),
            (b'<', Some(b'=')) => Some((TokenKind::Le, 2)),
            (b'<', _) => Some((TokenKind::Lt, 1)),
            (b'&', Some(b'&')) => Some((TokenKind::And, 2)),
            (b'|', Some(b'|')) => Some((TokenKind::Or, 2)),
            _ => None,
        };
        if let Some((kind, len)) = punct {
            i += len;
            tokens.push(Token {
                kind,
                pos: start,
                end: i,
            });
            continue;
        }

        if matches!(b, b'"' | b'\'' | b'`') {
            let (buf, end) = scan_quoted(input, start, b as char)?;
            i = end;
            let kind = if b == b'`' {
                TokenKind::Backtick(buf)
            } else {
                TokenKind::Str(buf)
            };
            tokens.push(Token {
                kind,
                pos: start,
                end,
            });
            continue;
        }

        let after_adjacent_dot = tokens
            .last()
            .is_some_and(|t: &Token| t.kind == TokenKind::Dot && t.end == start);

        if !after_adjacent_dot
            && (b.is_ascii_digit() || (b == b'-' && next.is_some_and(|c| c.is_ascii_digit())))
        {
            i += 1;
            let mut has_dot = false;
            while i < bytes.len() {
                let c = bytes[i];
                if c.is_ascii_digit() {
                    i += 1;
                } else if c == b'.' && !has_dot && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                {
                    has_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }
            let raw = &input[start..i];
            let kind = if has_dot {
                let f = raw
                    .parse::<f64>()
                    .map_err(|_| FilterError::UnexpectedToken {
                        token: raw.to_string(),
                        pos: start,
                    })?;
                TokenKind::Float(f)
            } else {
                let n = raw
                    .parse::<i64>()
                    .map_err(|_| FilterError::UnexpectedToken {
                        token: raw.to_string(),
                        pos: start,
                    })?;
                TokenKind::Int(n)
            };
            tokens.push(Token {
                kind,
                pos: start,
                end: i,
            });
            continue;
        }

        // Identifier segment (allows alphanumerics, `_`, `-`; dots are tokenized separately).
        let mut end = i;
        for (rel, ch) in input[i..].char_indices() {
            if ch.is_alphanumeric() || matches!(ch, '_' | '-') {
                end = i + rel + ch.len_utf8();
            } else {
                break;
            }
        }
        if end == i {
            let ch = input[i..].chars().next().unwrap_or('?');
            return Err(FilterError::UnexpectedToken {
                token: ch.to_string(),
                pos: start,
            });
        }
        let word = &input[start..end];
        i = end;
        let kind = if after_adjacent_dot {
            TokenKind::Ident(word.to_string())
        } else {
            match word {
                "true" => TokenKind::Bool(true),
                "false" => TokenKind::Bool(false),
                "null" => TokenKind::Null,
                "and" | "AND" => TokenKind::And,
                "or" | "OR" => TokenKind::Or,
                "not" | "NOT" => TokenKind::Not,
                "contains" | "CONTAINS" => TokenKind::Contains,
                _ => TokenKind::Ident(word.to_string()),
            }
        };
        tokens.push(Token {
            kind,
            pos: start,
            end,
        });
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.idx)
    }

    fn next(&mut self) -> Option<Token> {
        if self.idx < self.tokens.len() {
            let tok = self.tokens[self.idx].clone();
            self.idx += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn parse_or(&mut self) -> Result<FilterExpr, FilterError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Or)) {
            self.next();
            let right = self.parse_and()?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, FilterError> {
        let mut left = self.parse_not()?;
        while matches!(self.peek().map(|t| &t.kind), Some(TokenKind::And)) {
            self.next();
            let right = self.parse_not()?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<FilterExpr, FilterError> {
        if matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Not)) {
            self.next();
            let inner = self.parse_not()?;
            return Ok(FilterExpr::Not(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<FilterExpr, FilterError> {
        let tok = self.next().ok_or(FilterError::UnexpectedEof)?;
        match tok.kind {
            TokenKind::LParen => {
                let expr = self.parse_or()?;
                match self.next() {
                    Some(Token {
                        kind: TokenKind::RParen,
                        ..
                    }) => Ok(expr),
                    Some(other) => Err(FilterError::UnexpectedToken {
                        token: format!("{:?}", other.kind),
                        pos: other.pos,
                    }),
                    None => Err(FilterError::UnexpectedEof),
                }
            }
            TokenKind::Ident(_)
            | TokenKind::Str(_)
            | TokenKind::Backtick(_)
            | TokenKind::LBracket => {
                let meta_field = if !matches!(
                    self.peek().map(|t| &t.kind),
                    Some(TokenKind::Dot | TokenKind::LBracket)
                ) {
                    match &tok.kind {
                        TokenKind::Ident(s) if s == "has_frontmatter" => {
                            Some(MetaField::HasFrontmatter)
                        }
                        TokenKind::Ident(s) if s == "frontmatter_format" => {
                            Some(MetaField::FrontmatterFormat)
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                let op_for = |parser: &Parser| match parser.peek().map(|t| &t.kind) {
                    Some(TokenKind::Eq) => Some(CompareOp::Eq),
                    Some(TokenKind::Ne) => Some(CompareOp::Ne),
                    Some(TokenKind::Gt) => Some(CompareOp::Gt),
                    Some(TokenKind::Ge) => Some(CompareOp::Ge),
                    Some(TokenKind::Lt) => Some(CompareOp::Lt),
                    Some(TokenKind::Le) => Some(CompareOp::Le),
                    Some(TokenKind::Contains) => Some(CompareOp::Contains),
                    _ => None,
                };

                if let Some(field) = meta_field {
                    if let Some(op) = op_for(self) {
                        self.next();
                        let value = self.parse_literal()?;
                        Ok(FilterExpr::MetaCompare { field, op, value })
                    } else {
                        Ok(FilterExpr::MetaTruthy(field))
                    }
                } else {
                    let path = self.parse_path(tok)?;
                    if let Some(op) = op_for(self) {
                        self.next();
                        let value = self.parse_literal()?;
                        Ok(FilterExpr::Compare { path, op, value })
                    } else {
                        Ok(FilterExpr::Truthy(path))
                    }
                }
            }
            other => Err(FilterError::UnexpectedToken {
                token: format!("{other:?}"),
                pos: tok.pos,
            }),
        }
    }

    fn parse_path(&mut self, first: Token) -> Result<Vec<String>, FilterError> {
        let head = match first.kind {
            TokenKind::Ident(s) => s,
            TokenKind::Str(s) | TokenKind::Backtick(s) if !s.is_empty() => s,
            TokenKind::LBracket => self.parse_bracket_segment()?,
            other => {
                return Err(FilterError::UnexpectedToken {
                    token: format!("{other:?}"),
                    pos: first.pos,
                });
            }
        };

        let mut segments = vec![head];
        loop {
            match self.peek().map(|t| &t.kind) {
                Some(TokenKind::Dot) => {
                    self.next();
                    let seg_tok = self.next().ok_or(FilterError::UnexpectedEof)?;
                    let seg = match seg_tok.kind {
                        TokenKind::Ident(s) => s,
                        TokenKind::Str(s) | TokenKind::Backtick(s) if !s.is_empty() => s,
                        TokenKind::Int(n) if n >= 0 => n.to_string(),
                        other => {
                            return Err(FilterError::UnexpectedToken {
                                token: format!("{other:?}"),
                                pos: seg_tok.pos,
                            });
                        }
                    };
                    segments.push(seg);
                }
                Some(TokenKind::LBracket) => {
                    self.next();
                    segments.push(self.parse_bracket_segment()?);
                }
                _ => break,
            }
        }

        Ok(segments)
    }

    fn parse_bracket_segment(&mut self) -> Result<String, FilterError> {
        let inner = self.next().ok_or(FilterError::UnexpectedEof)?;
        let seg = match inner.kind {
            TokenKind::Str(s) if !s.is_empty() => s,
            TokenKind::Int(n) if n >= 0 => n.to_string(),
            other => {
                return Err(FilterError::UnexpectedToken {
                    token: format!("{other:?}"),
                    pos: inner.pos,
                });
            }
        };
        match self.next() {
            Some(Token {
                kind: TokenKind::RBracket,
                ..
            }) => Ok(seg),
            Some(other) => Err(FilterError::UnexpectedToken {
                token: format!("{:?}", other.kind),
                pos: other.pos,
            }),
            None => Err(FilterError::UnexpectedEof),
        }
    }

    fn parse_literal(&mut self) -> Result<JsonValue, FilterError> {
        let tok = self.next().ok_or(FilterError::UnexpectedEof)?;
        match tok.kind {
            TokenKind::Str(s) => Ok(JsonValue::String(s)),
            TokenKind::Ident(mut s) => {
                let mut end = tok.end;
                while let Some(dot) = self.peek() {
                    if dot.kind != TokenKind::Dot || dot.pos != end {
                        break;
                    }
                    let dot_end = dot.end;
                    self.next();
                    match self.next() {
                        Some(Token {
                            kind: TokenKind::Ident(seg),
                            pos,
                            end: seg_end,
                        }) if pos == dot_end => {
                            s.push('.');
                            s.push_str(&seg);
                            end = seg_end;
                        }
                        Some(other) => {
                            return Err(FilterError::UnexpectedToken {
                                token: format!("{:?}", other.kind),
                                pos: other.pos,
                            });
                        }
                        None => return Err(FilterError::UnexpectedEof),
                    }
                }
                Ok(JsonValue::String(s))
            }
            TokenKind::Int(i) => Ok(JsonValue::Number(i.into())),
            TokenKind::Float(f) => serde_json::Number::from_f64(f)
                .map(JsonValue::Number)
                .ok_or(FilterError::UnexpectedToken {
                    token: f.to_string(),
                    pos: tok.pos,
                }),
            TokenKind::Bool(b) => Ok(JsonValue::Bool(b)),
            TokenKind::Null => Ok(JsonValue::Null),
            other => Err(FilterError::UnexpectedToken {
                token: format!("{other:?}"),
                pos: tok.pos,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn filter_equality_nested_and_contains() {
        let doc = json!({
            "title": "Rust Frontmatter Guide",
            "draft": true,
            "weight": 10,
            "author": { "name": "Nick" },
            "tags": ["rust", "cli", "markdown"]
        });

        assert!(FilterExpr::parse("draft == true").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("draft == false").unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#"author.name == "Nick""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"tags contains "rust""#)
            .unwrap()
            .matches(&doc));
        assert!(!FilterExpr::parse(r#"tags contains "python""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"title contains "Frontmatter""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse("weight >= 10 && draft")
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(
            r#"(draft == false or weight == 10) and not (author.name == "Bob")"#
        )
        .unwrap()
        .matches(&doc));
    }

    #[test]
    fn filter_invalid_expressions_error() {
        assert!(FilterExpr::parse("").is_err());
        assert!(FilterExpr::parse("draft ==").is_err());
        assert!(FilterExpr::parse(r#"title == "unterminated"#).is_err());
        assert!(FilterExpr::parse("(draft == true").is_err());
        assert!(FilterExpr::parse("draft == true extra").is_err());
        assert!(FilterExpr::parse("(draft == true extra").is_err());
        assert!(FilterExpr::parse("== true").is_err());
        assert!(FilterExpr::parse("draft == (true)").is_err());
        assert!(FilterExpr::parse("title == \"bad\\").is_err());
        assert!(FilterExpr::parse("draft @ true").is_err());
        assert!(FilterExpr::parse("weight == 99999999999999999999999999999").is_err());
    }

    #[test]
    fn filter_all_operators_types_and_escapes() {
        let doc = json!({
            "title": "Alpha Guide",
            "empty_str": "",
            "draft": false,
            "count": 0,
            "score": 2.5,
            "neg": -10,
            "date": "2026-06-15",
            "note": null,
            "tags": ["rust", "cli"],
            "empty_arr": [],
            "meta": { "k": "v" },
            "empty_obj": {},
            "escaped": "line1\nline2\r\t\\\"'\\x"
        });

        // Truthiness across all JSON value kinds.
        assert!(!FilterExpr::parse("note").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("draft").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("count").unwrap().matches(&doc));
        assert!(FilterExpr::parse("score").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("empty_str").unwrap().matches(&doc));
        assert!(FilterExpr::parse("title").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("empty_arr").unwrap().matches(&doc));
        assert!(FilterExpr::parse("tags").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("empty_obj").unwrap().matches(&doc));
        assert!(FilterExpr::parse("meta").unwrap().matches(&doc));
        assert!(FilterExpr::parse("!draft").unwrap().matches(&doc));
        assert!(FilterExpr::parse("NOT draft").unwrap().matches(&doc));

        // Array indexing and non-traversable paths.
        assert!(FilterExpr::parse("tags.0 == 'rust'").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("tags.bad == 'rust'")
            .unwrap()
            .matches(&doc));
        assert!(!FilterExpr::parse("title.sub == 'x'").unwrap().matches(&doc));

        // Numeric & string ordering, equality, inequality, null, floats, negatives.
        assert!(FilterExpr::parse("score == 2.5").unwrap().matches(&doc));
        assert!(FilterExpr::parse("neg == -10").unwrap().matches(&doc));
        assert!(FilterExpr::parse("note == null").unwrap().matches(&doc));
        assert!(FilterExpr::parse("title != 'Other'").unwrap().matches(&doc));
        assert!(FilterExpr::parse("missing != 'Other'")
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse("score > 2.0 && score < 3.0")
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse("score <= 2.5 || draft == true")
            .unwrap()
            .matches(&doc));
        assert!(
            FilterExpr::parse("date >= '2026-01-01' AND date < '2027-01-01'")
                .unwrap()
                .matches(&doc)
        );
        assert!(FilterExpr::parse("draft == true OR neg <= -10")
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse("tags CONTAINS cli")
            .unwrap()
            .matches(&doc));

        // Incompatible ordering & contains types evaluate to false.
        assert!(!FilterExpr::parse("title > 5").unwrap().matches(&doc));
        assert!(!FilterExpr::parse("count contains 0").unwrap().matches(&doc));

        // String escape sequences in double-quoted literal.
        assert!(FilterExpr::parse(r#"escaped == "line1\nline2\r\t\\\"'\x""#)
            .unwrap()
            .matches(&doc));
    }

    #[test]
    fn filter_quoted_backtick_and_bracket_keys_with_spaces() {
        let doc = json!({
            "prep time": "10 minutes",
            "cook time": "20 minutes",
            "total time": "30 minutes",
            "empty time": "",
            "null time": null
        });

        // Quoted LHS paths (double and single quotes).
        assert!(FilterExpr::parse(r#""prep time" != null"#)
            .unwrap()
            .matches(&doc));
        assert!(!FilterExpr::parse(r#""null time" != null"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#""prep time" == "10 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"'cook time' == "20 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"'total time' contains "30""#)
            .unwrap()
            .matches(&doc));

        // Backtick-quoted key paths.
        assert!(FilterExpr::parse("`prep time` != null")
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"`cook time` == "20 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"`total time` == "30 minutes""#)
            .unwrap()
            .matches(&doc));

        // Bracket notation at root.
        assert!(FilterExpr::parse(r#"["prep time"] != null"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"['cook time'] == "20 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"["total time"] contains "minutes""#)
            .unwrap()
            .matches(&doc));

        // Truthiness, negation, and boolean composition.
        assert!(FilterExpr::parse(r#""prep time""#).unwrap().matches(&doc));
        assert!(!FilterExpr::parse(r#""empty time""#).unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#"!"empty time""#).unwrap().matches(&doc));
        assert!(FilterExpr::parse("`prep time`").unwrap().matches(&doc));
        assert!(FilterExpr::parse("not `empty time`").unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#"["prep time"]"#).unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#"!["null time"]"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(
            r#"("prep time" != null && 'cook time' == "20 minutes") and not `empty time`"#
        )
        .unwrap()
        .matches(&doc));
    }

    #[test]
    fn filter_nested_punctuation_and_indexed_paths() {
        let doc = json!({
            "version": "v1.2",
            "meta": {
                "prep time": "5 minutes",
                "and": "yes",
                "null": "ok",
                "true": 42
            },
            "recipe info": {
                "prep time": "5 minutes",
                "author": { "name": "Nick" }
            },
            "a.b": { "c": 1 },
            "prep/cook time": "15m",
            "cost ($)": 12,
            "has \"quote\"": true,
            "it's ready": true,
            "has `tick`": true,
            "and": 1,
            "true": false,
            "null": "present",
            "tags": ["rust", "cli"],
            "items": [{ "name": "Bread", "prep time": "5 minutes" }],
            "matrix": [["a", "b"]]
        });

        // Dotted + bracketed / quoted / backticked combinations.
        assert!(FilterExpr::parse(r#"meta["prep time"] == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"meta['prep time'] == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"meta."prep time" == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"meta.'prep time' == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"meta.`prep time` == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(
            FilterExpr::parse(r#"["recipe info"]["prep time"] == "5 minutes""#)
                .unwrap()
                .matches(&doc)
        );
        assert!(
            FilterExpr::parse(r#"["recipe info"].author.name == "Nick""#)
                .unwrap()
                .matches(&doc)
        );
        assert!(
            FilterExpr::parse(r#"`recipe info`.`prep time` == "5 minutes""#)
                .unwrap()
                .matches(&doc)
        );
        assert!(
            FilterExpr::parse(r#"`recipe info`["prep time"] == "5 minutes""#)
                .unwrap()
                .matches(&doc)
        );
        assert!(
            FilterExpr::parse(r#""recipe info"."prep time" == "5 minutes""#)
                .unwrap()
                .matches(&doc)
        );

        // Array indexing via brackets and multi-level dot indices.
        assert!(FilterExpr::parse(r#"tags[0] == "rust""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"items[0]["prep time"] == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"items[0].name == "Bread""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"items.0["prep time"] == "5 minutes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"matrix.0.1 == "b""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"matrix[0][1] == "b""#)
            .unwrap()
            .matches(&doc));

        // Literal dots, punctuation, escaped quotes/backticks, and keywords as keys.
        assert!(FilterExpr::parse(r#"["a.b"].c == 1"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse("`a.b`.c == 1").unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#""a.b".c == 1"#).unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#""prep/cook time" == "15m""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#""cost ($)" == 12"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"["has \"quote\""] == true"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"'it\'s ready' == true"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"`has \`tick\`` == true"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"["and"] == 1"#).unwrap().matches(&doc));
        assert!(FilterExpr::parse("`true` == false").unwrap().matches(&doc));
        assert!(FilterExpr::parse(r#""null" != null"#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"meta.and == "yes""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse(r#"meta.null == "ok""#)
            .unwrap()
            .matches(&doc));
        assert!(FilterExpr::parse("meta.true == 42").unwrap().matches(&doc));

        // Dotted unquoted bareword on RHS remains valid.
        assert!(FilterExpr::parse("version == v1.2").unwrap().matches(&doc));
    }

    #[test]
    fn filter_invalid_quoted_and_bracket_expressions_error() {
        // Empty key segments.
        assert!(FilterExpr::parse(r#""""#).is_err());
        assert!(FilterExpr::parse(r#""" == 1"#).is_err());
        assert!(FilterExpr::parse("`` == 1").is_err());
        assert!(FilterExpr::parse(r#"[""] == 1"#).is_err());
        assert!(FilterExpr::parse(r#"meta[""] == 1"#).is_err());
        assert!(FilterExpr::parse(r#"meta."" == 1"#).is_err());
        assert!(FilterExpr::parse("meta.`` == 1").is_err());

        // Unterminated backticks.
        assert_eq!(
            FilterExpr::parse("`prep time"),
            Err(FilterError::UnterminatedQuotedKey { pos: 0 })
        );
        assert_eq!(
            FilterExpr::parse(r#"`bad\"#),
            Err(FilterError::UnterminatedQuotedKey { pos: 0 })
        );

        // Malformed brackets.
        assert!(FilterExpr::parse("[]").is_err());
        assert!(FilterExpr::parse(r#"["prep time""#).is_err());
        assert!(FilterExpr::parse("[prep]").is_err());
        assert!(FilterExpr::parse("tags[-1]").is_err());
        assert!(FilterExpr::parse("tags[1.5]").is_err());
        assert!(FilterExpr::parse("tags[true]").is_err());
        assert!(FilterExpr::parse(r#"["prep time" "extra"]"#).is_err());

        // Malformed dots and RHS tokens.
        assert!(FilterExpr::parse(".meta").is_err());
        assert!(FilterExpr::parse("meta.").is_err());
        assert!(FilterExpr::parse("meta..name").is_err());
        assert!(FilterExpr::parse("meta. == 1").is_err());
        assert!(FilterExpr::parse("meta. and draft").is_err());
        assert!(FilterExpr::parse(r#"meta.["prep time"]"#).is_err());
        assert!(FilterExpr::parse("title == `foo`").is_err());
        assert!(FilterExpr::parse(r#"title == ["foo"]"#).is_err());
        assert!(FilterExpr::parse(r#""prep time" "cook time""#).is_err());
        assert!(FilterExpr::parse("version == v1.").is_err());
        assert!(FilterExpr::parse("version == v1. and draft").is_err());
    }

    #[test]
    fn filter_has_frontmatter_and_frontmatter_format() {
        let yaml_doc = json!({ "draft": true, "has_frontmatter": "literal" });
        let empty_doc = json!({});

        let body_only = FilterContext {
            frontmatter: None,
            format: None,
        };
        let empty_yaml = FilterContext {
            frontmatter: Some(&empty_doc),
            format: Some(Format::Yaml),
        };
        let yaml_ctx = FilterContext {
            frontmatter: Some(&yaml_doc),
            format: Some(Format::Yaml),
        };
        let toml_ctx = FilterContext {
            frontmatter: Some(&empty_doc),
            format: Some(Format::Toml),
        };

        // Bare and negated `has_frontmatter`.
        let has_fm = FilterExpr::parse("has_frontmatter").unwrap();
        assert!(!has_fm.matches_doc(body_only));
        assert!(has_fm.matches_doc(empty_yaml));
        assert!(has_fm.matches_doc(yaml_ctx));

        let not_has_fm = FilterExpr::parse("not has_frontmatter").unwrap();
        assert!(not_has_fm.matches_doc(body_only));
        assert!(!not_has_fm.matches_doc(empty_yaml));

        let eq_false = FilterExpr::parse("has_frontmatter == false").unwrap();
        assert!(eq_false.matches_doc(body_only));
        assert!(!eq_false.matches_doc(empty_yaml));

        // `frontmatter_format` comparisons.
        let is_yaml = FilterExpr::parse(r#"frontmatter_format == "yaml""#).unwrap();
        assert!(!is_yaml.matches_doc(body_only));
        assert!(is_yaml.matches_doc(empty_yaml));
        assert!(!is_yaml.matches_doc(toml_ctx));

        let is_toml = FilterExpr::parse(r#"frontmatter_format == "toml""#).unwrap();
        assert!(is_toml.matches_doc(toml_ctx));
        assert!(!is_toml.matches_doc(yaml_ctx));

        let fmt_null = FilterExpr::parse("frontmatter_format == null").unwrap();
        assert!(fmt_null.matches_doc(body_only));
        assert!(!fmt_null.matches_doc(empty_yaml));

        // Combined with normal key predicates.
        let combined = FilterExpr::parse("not has_frontmatter or draft == true").unwrap();
        assert!(combined.matches_doc(body_only));
        assert!(!combined.matches_doc(empty_yaml));
        assert!(combined.matches_doc(yaml_ctx));

        // Backward compatibility: non-meta predicates still skip body-only files.
        assert!(!FilterExpr::parse("not draft")
            .unwrap()
            .matches_doc(body_only));
        assert!(!FilterExpr::parse("draft != true")
            .unwrap()
            .matches_doc(body_only));

        // Quoted `"has_frontmatter"` queries literal frontmatter key, not metadata.
        let quoted = FilterExpr::parse(r#""has_frontmatter" == "literal""#).unwrap();
        assert!(!quoted.references_document_meta());
        assert!(quoted.matches_doc(yaml_ctx));
        assert!(!quoted.matches_doc(body_only));
    }
}
