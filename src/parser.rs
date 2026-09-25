use crate::{Diagnostic, Document, Element, Limits, Node, Position, Variable};
use std::collections::{BTreeMap, BTreeSet};
use xmlparser::{ElementEnd, Token, Tokenizer};

/// Parse an integer token budget or a decimal-k budget: `500`, `10k`, `1.5K`.
/// k means 1000. Fractional tokens, negatives, whitespace, and overflow fail.
pub fn parse_limit(value: &str) -> Result<u64, String> {
    let error = || format!("Invalid token limit {value:?}; use 500, 10k, or 1.5k");
    let parse_digits = |s: &str| -> Result<u64, String> {
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(error());
        }
        s.parse::<u64>().map_err(|_| error())
    };
    if let Some(number) = value.strip_suffix('k').or_else(|| value.strip_suffix('K')) {
        let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
        let mut n = parse_digits(whole)?.checked_mul(1000).ok_or_else(error)?;
        if number.contains('.') {
            if fraction.is_empty() || fraction.len() > 3 {
                return Err(error());
            }
            n = n
                .checked_add(parse_digits(fraction)? * 10u64.pow(3 - fraction.len() as u32))
                .ok_or_else(error)?;
        }
        Ok(n)
    } else {
        parse_digits(value)
    }
}
struct Locations<'a> {
    source: &'a str,
    starts: Vec<usize>,
    wide: Vec<(usize, usize)>,
}
impl<'a> Locations<'a> {
    fn new(source: &'a str) -> Self {
        let mut extra = 0;
        let wide = source
            .char_indices()
            .filter_map(|(i, c)| {
                if c.is_ascii() {
                    None
                } else {
                    extra += c.len_utf8() - 1;
                    Some((i + c.len_utf8(), extra))
                }
            })
            .collect();
        Self {
            source,
            starts: std::iter::once(0)
                .chain(source.match_indices('\n').map(|(i, _)| i + 1))
                .collect(),
            wide,
        }
    }
    fn extra_bytes(&self, offset: usize) -> usize {
        let n = self.wide.partition_point(|&(end, _)| end <= offset);
        if n == 0 { 0 } else { self.wide[n - 1].1 }
    }
    fn at(&self, offset: usize) -> Position {
        let row = self.starts.partition_point(|&i| i <= offset) - 1;
        Position {
            line: row + 1,
            column: offset - self.starts[row] + 1
                - (self.extra_bytes(offset) - self.extra_bytes(self.starts[row])),
            offset,
        }
    }
    fn error(&self, row: usize, column: usize) -> Position {
        let start = self
            .starts
            .get(row.saturating_sub(1))
            .copied()
            .unwrap_or(self.source.len());
        let offset = self.source[start..]
            .char_indices()
            .nth(column.saturating_sub(1))
            .map_or(self.source.len(), |(i, _)| start + i);
        Position {
            line: row,
            column,
            offset,
        }
    }
}
pub(crate) fn valid_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= '\u{20}' && c != '\u{fffe}' && c != '\u{ffff}')
}
fn decode(value: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index + 1..];
        let end = rest
            .find(';')
            .ok_or("Unclosed entity; escape a literal & as &amp;")?;
        let entity = &rest[..end];
        let ch = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let n = if let Some(hex) = entity.strip_prefix("#x") {
                    (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()))
                        .then(|| u32::from_str_radix(hex, 16).ok())
                        .flatten()
                } else if let Some(dec) = entity.strip_prefix('#') {
                    (!dec.is_empty() && dec.bytes().all(|b| b.is_ascii_digit()))
                        .then(|| dec.parse::<u32>().ok())
                        .flatten()
                } else {
                    None
                };
                n.and_then(char::from_u32)
                    .filter(|c| valid_char(*c) && !('\u{80}'..='\u{9f}').contains(c))
                    .ok_or_else(|| format!("Unsupported entity &{entity};"))?
            }
        };
        out.push(ch);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out.replace("\r\n", "\n").replace('\r', "\n"))
}
pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
}
/// Lowercase ASCII names; the htmlp name is reserved for the document root.
pub(crate) fn valid_element_name(name: &str) -> bool {
    name != "htmlp"
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

struct Frame {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
    position: Position,
}
fn limits(frame: &Frame) -> Result<Limits, Diagnostic> {
    let get = |key: &str| {
        frame
            .attrs
            .get(key)
            .map(|s| parse_limit(s).map_err(|e| Diagnostic::new("limit", e, frame.position)))
            .transpose()
    };
    let limits = Limits {
        max_tokens: get("max-tokens")?,
        per_item: get("per-item")?,
        reason: frame.attrs.get("reason").cloned(),
        sig: frame.attrs.get("sig").cloned(),
    };
    if (limits.max_tokens.is_some() || limits.per_item.is_some())
        && limits.reason.as_ref().is_none_or(|r| r.trim().is_empty())
    {
        return Err(Diagnostic::new(
            "reason",
            "Every declared token limit requires a nonblank reason",
            frame.position,
        ));
    }
    Ok(limits)
}
fn append(nodes: &mut Vec<Node>, text: String) {
    if let Some(Node::Text { value }) = nodes.last_mut() {
        value.push_str(&text);
    } else if !text.is_empty() {
        nodes.push(Node::Text { value: text });
    }
}

/// Parse strict HTML-inspired markup using a tested XML tokenizer. XML-only
/// constructs, namespaces, invalid names, unknown attributes, and implicit repair are rejected.
/// Empty elements may use self-closing tags. Markdown indentation is preserved, not inferred or rewritten.
pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    if source.len() > 4 * 1024 * 1024 {
        return Err(Diagnostic::new(
            "source-size",
            "Source exceeds 4 MiB",
            Position::default(),
        ));
    }
    let locations = Locations::new(source);
    if let Some((i, _)) = source.char_indices().find(|(_, c)| !valid_char(*c)) {
        return Err(Diagnostic::new(
            "character",
            "Invalid control character",
            locations.at(i),
        ));
    }
    let mut stack: Vec<Frame> = Vec::new();
    let mut pending: Option<Frame> = None;
    let mut result = None;
    let mut ids = BTreeSet::new();
    let mut variables = BTreeSet::new();
    for token in Tokenizer::from(source) {
        let token = token.map_err(|e| {
            Diagnostic::new(
                "syntax",
                e.to_string(),
                locations.error(e.pos().row as usize, e.pos().col as usize),
            )
        })?;
        match token {
            Token::ElementStart {
                prefix,
                local,
                span,
            } => {
                let p = locations.at(span.start());
                if !prefix.is_empty()
                    || !(local.as_str() == "htmlp" || valid_element_name(local.as_str()))
                {
                    return Err(Diagnostic::new(
                        "element",
                        "Element names must start with a lowercase ASCII letter and contain only lowercase letters, digits, or hyphens",
                        p,
                    ));
                }
                if stack.len() >= 64 {
                    return Err(Diagnostic::new("depth", "Maximum element depth is 64", p));
                }
                if (stack.is_empty() && (local.as_str() != "htmlp" || result.is_some()))
                    || (!stack.is_empty() && local.as_str() == "htmlp")
                {
                    return Err(Diagnostic::new(
                        "root",
                        "Expected exactly one htmlp root",
                        p,
                    ));
                }
                pending = Some(Frame {
                    name: local.to_string(),
                    attrs: BTreeMap::new(),
                    children: Vec::new(),
                    position: p,
                });
            }
            Token::Attribute {
                prefix,
                local,
                value,
                span,
            } => {
                let frame = pending.as_mut().ok_or_else(|| {
                    Diagnostic::new("syntax", "Unexpected attribute", locations.at(span.start()))
                })?;
                let allowed: &[&str] = match frame.name.as_str() {
                    "htmlp" => &[
                        "version",
                        "tokenizer",
                        "max-tokens",
                        "per-item",
                        "reason",
                        "sig",
                    ],
                    _ => &["id", "max-tokens", "per-item", "reason", "sig"],
                };
                if !prefix.is_empty() || !allowed.contains(&local.as_str()) {
                    return Err(Diagnostic::new(
                        "attribute",
                        format!("Unknown attribute {local}"),
                        locations.at(span.start()),
                    ));
                }
                let decoded = decode(value.as_str())
                    .map_err(|e| Diagnostic::new("entity", e, locations.at(value.start())))?;
                if frame.attrs.insert(local.to_string(), decoded).is_some() {
                    return Err(Diagnostic::new(
                        "attribute",
                        "Duplicate attribute",
                        locations.at(span.start()),
                    ));
                }
            }
            Token::ElementEnd { end, span } => {
                if matches!(end, ElementEnd::Open | ElementEnd::Empty) {
                    let frame = pending.take().ok_or_else(|| {
                        Diagnostic::new(
                            "syntax",
                            "Unexpected opening tag",
                            locations.at(span.start()),
                        )
                    })?;
                    if let Some(id) = frame.attrs.get("id") {
                        if !valid_id(id) || variables.contains(id) || !ids.insert(id.clone()) {
                            return Err(Diagnostic::new(
                                "id",
                                "IDs must be unique and contain only ASCII letters, digits, _, -, ., or :",
                                frame.position,
                            ));
                        }
                    }
                    stack.push(frame);
                }
                if !matches!(end, ElementEnd::Open) {
                    let frame = stack.pop().ok_or_else(|| {
                        Diagnostic::new(
                            "syntax",
                            "Unexpected closing tag",
                            locations.at(span.start()),
                        )
                    })?;
                    if let ElementEnd::Close(prefix, local) = end {
                        if !prefix.is_empty() || frame.name != local.as_str() {
                            return Err(Diagnostic::new(
                                "syntax",
                                format!("Expected </{}>, found </{local}>", frame.name),
                                locations.at(span.start()),
                            ));
                        }
                    }
                    let budget = limits(&frame)?;
                    if frame.name == "htmlp" {
                        if frame.attrs.get("version").is_some_and(|v| v != "0.3") {
                            return Err(Diagnostic::new(
                                "version",
                                "Only version 0.3 is supported",
                                frame.position,
                            ));
                        }
                        if budget.max_tokens.is_none() {
                            return Err(Diagnostic::new(
                                "limit",
                                "htmlp requires max-tokens",
                                frame.position,
                            ));
                        }
                        let tokenizer = frame
                            .attrs
                            .get("tokenizer")
                            .cloned()
                            .unwrap_or_else(|| "cl100k_base".into());
                        if !valid_id(&tokenizer) {
                            return Err(Diagnostic::new(
                                "tokenizer",
                                "Invalid tokenizer identifier",
                                frame.position,
                            ));
                        }
                        result = Some(Document {
                            version: "0.3".into(),
                            tokenizer,
                            limits: budget,
                            children: frame.children,
                            position: frame.position,
                        });
                    } else {
                        let node = Node::Element(Element {
                            name: frame.name,
                            id: frame.attrs.get("id").cloned(),
                            limits: budget,
                            children: frame.children,
                            position: frame.position,
                        });
                        stack
                            .last_mut()
                            .ok_or_else(|| Diagnostic::new("root", "Missing root", frame.position))?
                            .children
                            .push(node);
                    }
                }
            }
            Token::Text { text } => {
                if let Some(frame) = stack.last_mut() {
                    let nodes = parse_text(text.as_str(), text.start(), &locations, true)?;
                    for node in nodes {
                        if let Node::Variable(variable) = &node {
                            if ids.contains(&variable.id) {
                                return Err(Diagnostic::new(
                                    "variable",
                                    "Variable name cannot match an element ID",
                                    variable.position,
                                ));
                            }
                            variables.insert(variable.id.clone());
                        }
                        match node {
                            Node::Text { value } => append(&mut frame.children, value),
                            node => frame.children.push(node),
                        }
                    }
                } else if !text.as_str().trim().is_empty() {
                    return Err(Diagnostic::new(
                        "root",
                        "Text must be inside htmlp",
                        locations.at(text.start()),
                    ));
                }
            }
            Token::Comment { .. } => {}
            _ => {
                return Err(Diagnostic::new(
                    "syntax",
                    "DOCTYPE, declarations, CDATA, and processing instructions are not supported",
                    Position::default(),
                ));
            }
        }
    }
    if !stack.is_empty() {
        return Err(Diagnostic::new(
            "syntax",
            "Unclosed element",
            stack.last().unwrap().position,
        ));
    }
    result.ok_or_else(|| Diagnostic::new("root", "Missing htmlp root", Position::default()))
}

/// Parse Rust text interpolation into the same nodes as markup placeholders.
/// Only {{name}} is special. Other text, including < and &, is literal; use
/// Node::text for literal braces. HTML source decodes entities separately.
pub fn parse_template(source: &str) -> Result<Vec<Node>, Diagnostic> {
    if source.len() > 4 * 1024 * 1024 {
        return Err(Diagnostic::new(
            "source-size",
            "Template exceeds 4 MiB",
            Position::default(),
        ));
    }
    let locations = Locations::new(source);
    if let Some((offset, _)) = source.char_indices().find(|(_, c)| !valid_char(*c)) {
        return Err(Diagnostic::new(
            "character",
            "Invalid control character",
            locations.at(offset),
        ));
    }
    parse_text(source, 0, &locations, false)
}
fn parse_text(
    raw: &str,
    offset: usize,
    locations: &Locations<'_>,
    entities: bool,
) -> Result<Vec<Node>, Diagnostic> {
    let literal = |start: usize, end: usize| {
        if entities {
            decode(&raw[start..end])
                .map_err(|e| Diagnostic::new("entity", e, locations.at(offset + start)))
        } else {
            Ok(raw[start..end].to_owned())
        }
    };
    let mut nodes = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = raw[cursor..].find("{{") {
        let start = cursor + relative;
        let position = locations.at(offset + start);
        let end = raw[start + 2..]
            .find("}}")
            .map(|i| start + 2 + i)
            .ok_or_else(|| {
                Diagnostic::new("variable", "Unclosed {{name}} placeholder", position)
            })?;
        let id = &raw[start + 2..end];
        if !valid_id(id) {
            return Err(Diagnostic::new(
                "variable",
                "Variable names must be valid IDs",
                position,
            ));
        }
        append(&mut nodes, literal(cursor, start)?);
        nodes.push(Node::Variable(Variable {
            id: id.into(),
            position,
        }));
        cursor = end + 2;
    }
    append(&mut nodes, literal(cursor, raw.len())?);
    Ok(nodes)
}
