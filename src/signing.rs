use crate::{Diagnostic, Document, Limits, Node, parse};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt::Write, ops::Range};
use xmlparser::{ElementEnd, Token, Tokenizer};

/// First eight bytes of SHA-256 as lowercase hex. Covers normalized local
/// limits and the decoded reason, not content, identity, or approval.
/// Returns `None` when neither limit is declared. See the specification for
/// the canonical byte encoding, allowing implementations in other languages.
pub fn budget_signature(limits: &Limits) -> Option<String> {
    if limits.max_tokens.is_none() && limits.per_item.is_none() {
        return None;
    }
    let mut hash = Sha256::new();
    hash.update(b"htmlp-budget-v1\0");
    for value in [limits.max_tokens, limits.per_item] {
        hash.update([u8::from(value.is_some())]);
        if let Some(value) = value {
            hash.update(value.to_be_bytes());
        }
    }
    let reason = limits.reason.as_deref().unwrap_or("");
    hash.update((reason.len() as u64).to_be_bytes());
    hash.update(reason.as_bytes());
    let mut sig = String::with_capacity(16);
    for byte in &hash.finalize()[..8] {
        write!(sig, "{byte:02x}").expect("writing to String cannot fail");
    }
    Some(sig)
}

impl Document {
    /// Explicitly accept the current budgets, updating every local signature.
    /// This does not validate the document or its budgets; call [`crate::lint`]
    /// afterwards. Variables never have signatures.
    pub fn sign(&mut self) {
        self.limits.sig = budget_signature(&self.limits);
        // Iterative traversal also supports trees built outside the parser.
        let mut pending: Vec<_> = self.children.iter_mut().collect();
        while let Some(node) = pending.pop() {
            if let Node::Section(section) = node {
                section.limits.sig = budget_signature(&section.limits);
                pending.extend(section.children.iter_mut());
            }
        }
    }
}

/// Add or replace budget signatures without reformatting source. Existing
/// quotes, Markdown, comments, line endings, and attribute order are preserved.
/// Removes signatures from elements that no longer declare a local limit.
/// Invalid syntax or missing reasons return an error without any output.
pub fn sign_source(source: &str) -> Result<String, Diagnostic> {
    let doc = parse(source)?;
    let mut signatures = BTreeMap::new();
    signatures.insert(doc.position.offset, budget_signature(&doc.limits));
    let mut pending: Vec<_> = doc.children.iter().collect();
    while let Some(node) = pending.pop() {
        if let Node::Section(section) = node {
            signatures.insert(section.position.offset, budget_signature(&section.limits));
            pending.extend(section.children.iter());
        }
    }
    let mut current = None;
    let mut insertion = 0;
    let mut attribute: Option<(Range<usize>, Range<usize>)> = None;
    let mut edits = Vec::new();
    // Parsing above guarantees valid tokens and an AST entry for each start.
    for token in Tokenizer::from(source) {
        match token.expect("source already parsed") {
            Token::ElementStart { span, .. } => {
                current = signatures.get(&span.start());
                attribute = None;
                insertion = span.end();
            }
            Token::Attribute {
                local, value, span, ..
            } => {
                insertion = span.end();
                if local.as_str() == "sig" {
                    attribute = Some((value.range(), span.range()));
                }
            }
            Token::ElementEnd {
                end: ElementEnd::Open | ElementEnd::Empty,
                ..
            } => match (current.expect("element has AST entry"), attribute.take()) {
                (Some(sig), Some((value, _))) => edits.push((value, sig.clone())),
                (Some(sig), None) => edits.push((insertion..insertion, format!(" sig=\"{sig}\""))),
                (None, Some((_, whole))) => edits.push((whole, String::new())),
                (None, None) => {}
            },
            _ => {}
        }
    }
    let mut output = source.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        output.replace_range(range, &replacement);
    }
    if output.len() > 4 * 1024 * 1024 {
        return Err(Diagnostic::new(
            "source-size",
            "Signed source exceeds 4 MiB",
            doc.position,
        ));
    }
    Ok(output)
}
