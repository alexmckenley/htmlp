use crate::{Bindings, Diagnostic, Document, Node, Position, Report, TokenCounter};
use std::{collections::BTreeMap, ops::Range};

/// A tokenizer-specific count of a complete rendered document, excluding
/// provider message framing. Distinct from a runtime request estimate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct TextMeasurement {
    pub tokenizer: String,
    pub tokens: u64,
}

/// A containing section, including unnamed sections and their source position.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct SectionOrigin {
    pub id: Option<String>,
    pub position: Position,
}

/// Non-overlapping UTF-8 byte span in the final text. Section ancestry preserves
/// nesting without counting a parent's text a second time. Variables remain
/// literal values and inherit their containing section's attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct RenderedSpan {
    pub range: Range<usize>,
    pub sections: Vec<SectionOrigin>,
    pub variable: Option<String>,
}

/// Immutable output of a complete successful validation. No constructor or
/// deserializer can fabricate checked output. Extracted text is ordinary text;
/// modifying it does not preserve this object's validation guarantee.
#[derive(Debug, Clone)]
pub struct RenderedPrompt {
    text: String,
    measurement: TextMeasurement,
    report: Report,
    spans: Vec<RenderedSpan>,
    sections: BTreeMap<String, Range<usize>>,
}
impl RenderedPrompt {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn measurement(&self) -> &TextMeasurement {
        &self.measurement
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn spans(&self) -> &[RenderedSpan] {
        &self.spans
    }
    /// Select from an already checked whole document. Ancestor and sibling
    /// budgets were validated before any section became available.
    pub fn section(&self, id: &str) -> Option<&str> {
        self.sections.get(id).map(|range| &self.text[range.clone()])
    }
    pub fn into_text(self) -> String {
        self.text
    }
}

/// Bind and validate the entire document, returning immutable text, exact
/// tokenizer measurement, budget report, and non-overlapping section provenance.
pub fn render_checked(
    doc: &Document,
    bindings: &Bindings,
    counter: &impl TokenCounter,
) -> Result<RenderedPrompt, Vec<Diagnostic>> {
    let (text, report) = crate::check::render_with_report(doc, bindings, counter)?;
    let mut output = RenderedPrompt {
        measurement: TextMeasurement {
            tokenizer: counter.name().into(),
            tokens: report
                .measurements
                .last()
                .and_then(|m| m.tokens)
                .expect("render counts root"),
        },
        text,
        report,
        spans: Vec::new(),
        sections: BTreeMap::new(),
    };
    fn walk(
        nodes: &[Node],
        bindings: &Bindings,
        ancestry: &mut Vec<SectionOrigin>,
        offset: &mut usize,
        out: &mut RenderedPrompt,
    ) {
        for node in nodes {
            let (length, variable) = match node {
                Node::Section(section) => {
                    let start = *offset;
                    ancestry.push(SectionOrigin {
                        id: section.id.clone(),
                        position: section.position,
                    });
                    walk(&section.children, bindings, ancestry, offset, out);
                    ancestry.pop();
                    if let Some(id) = &section.id {
                        out.sections.insert(id.clone(), start..*offset);
                    }
                    continue;
                }
                Node::Text { value } => (value.len(), None),
                Node::Variable(variable) => {
                    (bindings[&variable.id].len(), Some(variable.id.clone()))
                }
            };
            if length > 0 {
                out.spans.push(RenderedSpan {
                    range: *offset..*offset + length,
                    sections: ancestry.clone(),
                    variable,
                });
            }
            *offset += length;
        }
    }
    walk(
        &doc.children,
        bindings,
        &mut Vec::new(),
        &mut 0,
        &mut output,
    );
    Ok(output)
}
