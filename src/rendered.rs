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

/// A containing element, including anonymous elements and their source position.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct ElementOrigin {
    pub name: String,
    pub path: Vec<usize>,
    pub id: Option<String>,
    pub position: Position,
}

/// Non-overlapping UTF-8 byte span in the final text. Element ancestry preserves
/// nesting without counting a parent's text a second time. Variables remain
/// literal values and inherit their containing element's attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct RenderedSpan {
    pub range: Range<usize>,
    pub elements: Vec<ElementOrigin>,
    pub variable: Option<String>,
}

/// One element's rendered range, including empty and anonymous elements.
/// These ranges may overlap; use spans for a non-overlapping partition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
pub struct RenderedElement {
    pub origin: ElementOrigin,
    pub range: Range<usize>,
}

/// Immutable output of a complete successful validation. No constructor or
/// deserializer can fabricate checked output. Extracted text is ordinary text;
/// modifying it does not preserve this object's validation guarantee.
#[derive(Debug, Clone)]
pub struct RenderedDocument {
    text: String,
    measurement: TextMeasurement,
    report: Report,
    spans: Vec<RenderedSpan>,
    by_id: BTreeMap<String, Range<usize>>,
    elements: Vec<RenderedElement>,
}
impl RenderedDocument {
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
    /// budgets were validated before any element became available.
    pub fn element(&self, id: &str) -> Option<&str> {
        self.by_id.get(id).map(|range| &self.text[range.clone()])
    }
    pub fn elements(&self) -> &[RenderedElement] {
        &self.elements
    }
    pub fn elements_by_name(&self, name: &str) -> Vec<&str> {
        self.elements
            .iter()
            .filter(|e| e.origin.name == name)
            .map(|e| &self.text[e.range.clone()])
            .collect()
    }
    pub fn into_text(self) -> String {
        self.text
    }
}

/// Bind and validate the entire document, returning immutable text, exact
/// tokenizer measurement, budget report, and non-overlapping element provenance.
pub fn render_checked(
    doc: &Document,
    bindings: &Bindings,
    counter: &impl TokenCounter,
) -> Result<RenderedDocument, Vec<Diagnostic>> {
    let (text, report) = crate::check::render_with_report(doc, bindings, counter)?;
    let mut output = RenderedDocument {
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
        by_id: BTreeMap::new(),
        elements: Vec::new(),
    };
    fn walk(
        nodes: &[Node],
        bindings: &Bindings,
        ancestry: &mut Vec<ElementOrigin>,
        path: &mut Vec<usize>,
        offset: &mut usize,
        out: &mut RenderedDocument,
    ) {
        for (index, node) in nodes.iter().enumerate() {
            let (length, variable) = match node {
                Node::Element(section) => {
                    let start = *offset;
                    path.push(index);
                    let origin = ElementOrigin {
                        name: section.name.clone(),
                        id: section.id.clone(),
                        position: section.position,
                        path: path.clone(),
                    };
                    let item = out.elements.len();
                    out.elements.push(RenderedElement {
                        origin: origin.clone(),
                        range: start..start,
                    });
                    ancestry.push(origin);
                    walk(&section.children, bindings, ancestry, path, offset, out);
                    ancestry.pop();
                    path.pop();
                    out.elements[item].range.end = *offset;
                    if let Some(id) = &section.id {
                        out.by_id.insert(id.clone(), start..*offset);
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
                    elements: ancestry.clone(),
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
        &mut Vec::new(),
        &mut 0,
        &mut output,
    );
    Ok(output)
}
