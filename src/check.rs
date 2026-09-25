use crate::{Bindings, Diagnostic, Document, Limits, Measurement, Node, Position, Report};
use std::collections::BTreeMap;

/// Inject a tokenizer to keep the parser independent of large vocabulary tables.
/// `name` must match the document's tokenizer identifier. Implementations must
/// count the supplied string as ordinary text, without interpreting special IDs.
pub trait TokenCounter {
    fn name(&self) -> &str;
    fn count(&self, text: &str) -> u64;
}

enum Piece {
    Text(String),
    Unbound,
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
    // Adjacent literal runs are always merged. Without variables there is
    // either one complete string or an empty subtree.
    if pieces.iter().any(|p| matches!(p, Piece::Unbound)) {
        return None;
    }
    Some(
        pieces
            .iter()
            .map(|p| match p {
                Piece::Text(s) => counter.count(s),
                Piece::Unbound => 0,
            })
            .sum(),
    )
}

fn require_reason(limits: &Limits, position: Position, errors: &mut Vec<Diagnostic>) {
    if limits.sig != crate::budget_signature(limits) {
        errors.push(Diagnostic::new(
            "signature",
            "Missing or mismatched budget signature. If you intend to do this, please rerun `htmlp sign` to regenerate the signature.",
            position,
        ));
    }
    if (limits.max_tokens.is_some() || limits.per_item.is_some())
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
    let mut ids = BTreeMap::new();
    fn walk(
        nodes: &[Node],
        ids: &mut BTreeMap<String, bool>,
        depth: usize,
        errors: &mut Vec<Diagnostic>,
    ) {
        for node in nodes {
            if matches!(node, Node::Section(_)) && depth > 64 {
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
                Node::Variable(v) => (Some(v.id.as_str()), v.position),
            };
            if let Some(id) = id {
                let variable = matches!(node, Node::Variable(_));
                let previous = ids.insert(id.to_string(), variable);
                if !crate::parser::valid_id(id)
                    || previous.is_some_and(|was_variable| !(variable && was_variable))
                {
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
        if let (Some(tokens), Some(limit)) = (tokens, cap) {
            if tokens > limit {
                self.report.diagnostics.push(Diagnostic::new(
                    "budget",
                    format!(
                        "{}: {tokens} tokens exceeds {limit} — {}",
                        id.unwrap_or("document/section"),
                        reason.unwrap_or("inherited item limit")
                    ),
                    position,
                ));
            }
        }
        self.report.measurements.push(Measurement {
            id: id.map(str::to_string),
            tokens,
            limit: cap,
            deferred: tokens.is_none(),
            reason: reason.map(str::to_string),
        });
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
                        .per_item
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
                        Piece::Unbound
                    };
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

/// Check fully static subtrees exactly. Subtrees containing variables are
/// reported as deferred, with no guessed count. Render to check their budgets.
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

/// Validate static content, substitute inert strings, then check every final
/// file and section budget. Variables have no separate limit. No output is
/// returned on any error.
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
