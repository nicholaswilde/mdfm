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
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Ident(String),
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
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    pos: usize,
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

/// Parsed AST for `--where` filter expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Compare {
        path: String,
        op: CompareOp,
        value: JsonValue,
    },
    Truthy(String),
}

impl FilterExpr {
    /// Parse a filter expression such as `draft == true`, `author.name == "Nick"`,
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

    /// Evaluate the filter expression against a frontmatter JSON object.
    pub fn matches(&self, root: &JsonValue) -> bool {
        match self {
            Self::And(lhs, rhs) => lhs.matches(root) && rhs.matches(root),
            Self::Or(lhs, rhs) => lhs.matches(root) || rhs.matches(root),
            Self::Not(inner) => !inner.matches(root),
            Self::Truthy(path) => lookup_path(root, path).is_some_and(is_truthy),
            Self::Compare { path, op, value } => {
                let actual = lookup_path(root, path);
                evaluate_compare(actual, *op, value)
            }
        }
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

fn lookup_path<'a>(root: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let mut current = root;
    for seg in path.split('.') {
        match current {
            JsonValue::Object(map) => current = map.get(seg)?,
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
        match b {
            b'(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    pos: start,
                });
                i += 1;
            }
            b')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    pos: start,
                });
                i += 1;
            }
            b'=' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(Token {
                    kind: TokenKind::Eq,
                    pos: start,
                });
                i += 2;
            }
            b'!' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(Token {
                    kind: TokenKind::Ne,
                    pos: start,
                });
                i += 2;
            }
            b'!' => {
                tokens.push(Token {
                    kind: TokenKind::Not,
                    pos: start,
                });
                i += 1;
            }
            b'>' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(Token {
                    kind: TokenKind::Ge,
                    pos: start,
                });
                i += 2;
            }
            b'>' => {
                tokens.push(Token {
                    kind: TokenKind::Gt,
                    pos: start,
                });
                i += 1;
            }
            b'<' if bytes.get(i + 1) == Some(&b'=') => {
                tokens.push(Token {
                    kind: TokenKind::Le,
                    pos: start,
                });
                i += 2;
            }
            b'<' => {
                tokens.push(Token {
                    kind: TokenKind::Lt,
                    pos: start,
                });
                i += 1;
            }
            b'&' if bytes.get(i + 1) == Some(&b'&') => {
                tokens.push(Token {
                    kind: TokenKind::And,
                    pos: start,
                });
                i += 2;
            }
            b'|' if bytes.get(i + 1) == Some(&b'|') => {
                tokens.push(Token {
                    kind: TokenKind::Or,
                    pos: start,
                });
                i += 2;
            }
            b'"' | b'\'' => {
                let quote = b as char;
                i += 1;
                let mut buf = String::new();
                let mut closed = false;
                let mut chars = input[i..].char_indices();
                while let Some((rel, ch)) = chars.next() {
                    if ch == '\\' {
                        if let Some((_, esc)) = chars.next() {
                            match esc {
                                'n' => buf.push('\n'),
                                'r' => buf.push('\r'),
                                't' => buf.push('\t'),
                                '\\' => buf.push('\\'),
                                '"' => buf.push('"'),
                                '\'' => buf.push('\''),
                                other => {
                                    buf.push('\\');
                                    buf.push(other);
                                }
                            }
                        } else {
                            return Err(FilterError::UnterminatedString { pos: start });
                        }
                    } else if ch == quote {
                        i += rel + ch.len_utf8();
                        closed = true;
                        break;
                    } else {
                        buf.push(ch);
                    }
                }
                if !closed {
                    return Err(FilterError::UnterminatedString { pos: start });
                }
                tokens.push(Token {
                    kind: TokenKind::Str(buf),
                    pos: start,
                });
            }
            b'0'..=b'9' | b'-'
                if b.is_ascii_digit()
                    || (b == b'-' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)) =>
            {
                i += 1;
                let mut has_dot = false;
                while i < bytes.len() {
                    let c = bytes[i];
                    if c.is_ascii_digit() {
                        i += 1;
                    } else if c == b'.'
                        && !has_dot
                        && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
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
                tokens.push(Token { kind, pos: start });
            }
            _ => {
                // Identifier / dot-path (allows alphanumerics, `_`, `-`, `.`).
                let mut end = i;
                for (rel, ch) in input[i..].char_indices() {
                    if ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.') {
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
                let kind = match word {
                    "true" => TokenKind::Bool(true),
                    "false" => TokenKind::Bool(false),
                    "null" => TokenKind::Null,
                    "and" | "AND" => TokenKind::And,
                    "or" | "OR" => TokenKind::Or,
                    "not" | "NOT" => TokenKind::Not,
                    "contains" | "CONTAINS" => TokenKind::Contains,
                    _ => TokenKind::Ident(word.to_string()),
                };
                tokens.push(Token { kind, pos: start });
            }
        }
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
            TokenKind::Ident(path) => {
                let op = match self.peek().map(|t| &t.kind) {
                    Some(TokenKind::Eq) => Some(CompareOp::Eq),
                    Some(TokenKind::Ne) => Some(CompareOp::Ne),
                    Some(TokenKind::Gt) => Some(CompareOp::Gt),
                    Some(TokenKind::Ge) => Some(CompareOp::Ge),
                    Some(TokenKind::Lt) => Some(CompareOp::Lt),
                    Some(TokenKind::Le) => Some(CompareOp::Le),
                    Some(TokenKind::Contains) => Some(CompareOp::Contains),
                    _ => None,
                };

                if let Some(op) = op {
                    self.next();
                    let value = self.parse_literal()?;
                    Ok(FilterExpr::Compare { path, op, value })
                } else {
                    Ok(FilterExpr::Truthy(path))
                }
            }
            other => Err(FilterError::UnexpectedToken {
                token: format!("{other:?}"),
                pos: tok.pos,
            }),
        }
    }

    fn parse_literal(&mut self) -> Result<JsonValue, FilterError> {
        let tok = self.next().ok_or(FilterError::UnexpectedEof)?;
        match tok.kind {
            TokenKind::Str(s) | TokenKind::Ident(s) => Ok(JsonValue::String(s)),
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
}
