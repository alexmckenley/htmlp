use crate::{Bindings, Diagnostic, Document, Limits, Measurement, Node, Position, Report};
use std::collections::BTreeSet;

/// Inject a tokenizer to keep the parser independent of large vocabulary tables.
/// `name` must match the document's tokenizer identifier. Implementations must
/// count the supplied string as ordinary text, without interpreting special IDs.
pub trait TokenCounter {
    fn name(&self) -> &str;
    fn count(&self, text: &str) -> u64;
}

enum Piece {
    Text(String),
    Reserved(u64),
}
fn extend(into: &mut Vec<Piece>, from: Vec<Piece>) {
    for piece in from {
        match (into.last_mut(), piece) {
            (Some(Piece::Text(previous)), Piece::Text(text)) => previous.push_str(&text),
            (_, piece) => into.push(piece),
        }
    }
}
fn size(pieces: &[Piece], counter: &impl TokenCounter) -> Option<u64> {
    pieces.iter().try_fold(0u64, |n, p| {
        n.checked_add(match p {
            Piece::Text(s) => counter.count(s),
            Piece::Reserved(n) => *n,
        })
    })
}
fn require_reason(limits: &Limits, position: Position, errors: &mut Vec<Diagnostic>) {
    if (limits.max_tokens.is_some() || limits.max_item_tokens.is_some())
        && limits.reason.as_ref().is_none_or(|r| r.trim().is_empty())
    {
        errors.push(Diagnostic::new(
            "reason",
            "Every declared token limit requires a nonblank reason",
            position,
        ));
    }
}
fn validate(doc: &Document, counter: &impl TokenCounter) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    require_reason(&doc.limits, doc.position, &mut errors);
    if doc.version != "0.2" || doc.limits.max_tokens.is_none() {
        errors.push(Diagnostic::new(
            "document",
            "Expected version 0.2 and a root max-tokens limit",
            doc.position,
        ));
    }
    if doc.tokenizer != counter.name() {
        errors.push(Diagnostic::new(
            "tokenizer",
            format!(
                "Document uses {}, counter uses {}",
                doc.tokenizer,
                counter.name()
            ),
            doc.position,
        ));
    }
    let mut ids = BTreeSet::new();
    fn walk(
        nodes: &[Node],
        ids: &mut BTreeSet<String>,
        depth: usize,
        errors: &mut Vec<Diagnostic>,
    ) {
        for node in nodes {
            if !matches!(node, Node::Text { .. }) && depth > 64 {
                errors.push(Diagnostic::new(
                    "depth",
                    "Maximum element depth is 64",
                    Position::default(),
                ));
                return;
            }
            let (id, position) = match node {
                Node::Text { .. } => continue,
                Node::Section(s) => {
                    require_reason(&s.limits, s.position, errors);
                    walk(&s.children, ids, depth + 1, errors);
                    (s.id.as_deref(), s.position)
                }
                Node::Variable(v) => {
                    if v.reason.trim().is_empty() {
                        errors.push(Diagnostic::new(
                            "reason",
                            "A variable bound requires a nonblank reason",
                            v.position,
                        ));
                    }
                    (Some(v.id.as_str()), v.position)
                }
            };
            if let Some(id) = id {
                if !crate::parser::valid_id(id) || !ids.insert(id.to_string()) {
                    errors.push(Diagnostic::new(
                        "id",
                        format!("Invalid or duplicate ID: {id}"),
                        position,
                    ));
                }
            }
        }
    }
    walk(&doc.children, &mut ids, 2, &mut errors);
    errors
}
struct Analyzer<'a, C> {
    counter: &'a C,
    bindings: Option<&'a Bindings>,
    report: Report,
}
impl<C: TokenCounter> Analyzer<'_, C> {
    fn measure(
        &mut self,
        pieces: &[Piece],
        cap: Option<u64>,
        id: Option<&str>,
        position: Position,
        reason: Option<&str>,
    ) {
        let tokens = size(pieces, self.counter);
        let reserved = pieces.iter().any(|p| matches!(p, Piece::Reserved(_)));
        match tokens {
            None => self.report.diagnostics.push(Diagnostic::new(
                "overflow",
                "Token reservations overflow u64",
                position,
            )),
            Some(tokens) => {
                if cap.is_some_and(|limit| tokens > limit) {
                    self.report.diagnostics.push(Diagnostic::new(
                        "budget",
                        format!(
                            "{}: {tokens} tokens exceeds {} — {}",
                            id.unwrap_or("document/section"),
                            cap.unwrap(),
                            reason.unwrap_or("inherited item limit")
                        ),
                        position,
                    ));
                }
                self.report.measurements.push(Measurement {
                    id: id.map(str::to_string),
                    tokens,
                    limit: cap,
                    reserved,
                    reason: reason.map(str::to_string),
                });
            }
        }
    }
    fn content(
        &mut self,
        nodes: &[Node],
        limits: &Limits,
        id: Option<&str>,
        position: Position,
        inherited: Option<(u64, &str)>,
    ) -> Vec<Piece> {
        let mut pieces = Vec::new();
        for node in nodes {
            match node {
                Node::Text { value } => extend(&mut pieces, vec![Piece::Text(value.clone())]),
                Node::Section(s) => {
                    let inherited = limits
                        .max_item_tokens
                        .map(|cap| (cap, limits.reason.as_deref().unwrap_or_default()));
                    let child = self.content(
                        &s.children,
                        &s.limits,
                        s.id.as_deref(),
                        s.position,
                        inherited,
                    );
                    extend(&mut pieces, child);
                }
                Node::Variable(v) => {
                    let piece = if let Some(bindings) = self.bindings {
                        match bindings.get(&v.id) {
                            Some(value) => Piece::Text(value.clone()),
                            None => {
                                self.report.diagnostics.push(Diagnostic::new(
                                    "binding",
                                    format!("Missing string binding: {}", v.id),
                                    v.position,
                                ));
                                Piece::Text(String::new())
                            }
                        }
                    } else {
                        Piece::Reserved(v.max_tokens)
                    };
                    self.measure(
                        std::slice::from_ref(&piece),
                        Some(v.max_tokens),
                        Some(&v.id),
                        v.position,
                        Some(&v.reason),
                    );
                    extend(&mut pieces, vec![piece]);
                }
            }
        }
        let (cap, reason) = match inherited {
            Some((cap, reason)) if limits.max_tokens.is_none_or(|own| cap < own) => {
                (Some(cap), Some(reason))
            }
            _ => (limits.max_tokens, limits.reason.as_deref()),
        };
        self.measure(&pieces, cap, id, position, reason);
        pieces
    }
}

/// Check every declared budget. Static text is counted exactly after concatenation.
/// Unbound variables reserve their declared allowance; BPE is not additive at
/// interpolation boundaries, so final substituted content must also be checked.
pub fn lint(doc: &Document, counter: &impl TokenCounter) -> Report {
    let diagnostics = validate(doc, counter);
    if !diagnostics.is_empty() {
        return Report {
            diagnostics,
            measurements: Vec::new(),
        };
    }
    let mut analyzer = Analyzer {
        counter,
        bindings: None,
        report: Report::default(),
    };
    analyzer.content(&doc.children, &doc.limits, None, doc.position, None);
    analyzer.report
}

/// Validate static reservations, substitute inert strings, then recount every
/// final subtree and variable. No output is returned on any error.
pub fn render(
    doc: &Document,
    bindings: &Bindings,
    counter: &impl TokenCounter,
) -> Result<String, Vec<Diagnostic>> {
    let checked = lint(doc, counter);
    if !checked.is_ok() {
        return Err(checked.diagnostics);
    }
    let mut analyzer = Analyzer {
        counter,
        bindings: Some(bindings),
        report: Report::default(),
    };
    let pieces = analyzer.content(&doc.children, &doc.limits, None, doc.position, None);
    if !analyzer.report.is_ok() {
        return Err(analyzer.report.diagnostics);
    }
    Ok(pieces
        .into_iter()
        .filter_map(|p| {
            if let Piece::Text(s) = p {
                Some(s)
            } else {
                None
            }
        })
        .collect())
}
