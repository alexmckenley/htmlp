use std::{collections::BTreeMap, fmt};

/// One-based Unicode-scalar line/column; zero-based UTF-8 byte offset.
/// Zero line/column denotes an unknown location or an in-memory node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// Syntax, budget, tokenizer, or binding failure. No partial AST is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub position: Position,
}
impl Diagnostic {
    pub(crate) fn new(code: &str, message: impl Into<String>, position: Position) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            position,
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}: {}",
            self.position.line, self.position.column, self.code, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}

/// Limits are tokens, stored as expanded integers (`1.5k` becomes 1500).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json", serde(deny_unknown_fields))]
pub struct Limits {
    pub max_tokens: Option<u64>,
    /// Applies to each direct child section; does not change grandchildren.
    pub per_item: Option<u64>,
    /// Required, nonblank rationale whenever either limit is declared.
    pub reason: Option<String>,
}

/// Markdown stays literal text. These are the only content node kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "json",
    serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)
)]
pub enum Node {
    Text { value: String },
    Section(Section),
    Variable(Variable),
}

/// A semantic grouping. IDs have no provider-specific role or trust meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json", serde(deny_unknown_fields))]
pub struct Section {
    pub id: Option<String>,
    pub limits: Limits,
    pub children: Vec<Node>,
    pub position: Position,
}

/// A named string slot: `{{question}}`. Variables have no limits.
///
/// ```compile_fail
/// use htmlp::{Variable, Position};
/// let slot = Variable { id: "question".into(), max_tokens: 100, position: Position::default() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json", serde(deny_unknown_fields))]
pub struct Variable {
    pub id: String,
    pub position: Position,
}

/// Portable AST. Use [`crate::parse`] or [`Document::new`] to create a document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "json", serde(deny_unknown_fields))]
pub struct Document {
    pub version: String,
    pub tokenizer: String,
    pub limits: Limits,
    pub children: Vec<Node>,
    pub position: Position,
}

/// Runtime values are strings; never expressions or reparsed markup.
pub type Bindings = BTreeMap<String, String>;

/// A typed reference returned by ID lookup. Repeated variable names return
/// the first reference in source order.
#[derive(Debug, Clone, Copy)]
pub enum ElementRef<'a> {
    Section(&'a Section),
    Variable(&'a Variable),
}

fn lookup<'a>(nodes: &'a [Node], id: &str) -> Option<ElementRef<'a>> {
    for node in nodes {
        match node {
            Node::Section(s) => {
                if s.id.as_deref() == Some(id) {
                    return Some(ElementRef::Section(s));
                }
                if let Some(found) = lookup(&s.children, id) {
                    return Some(found);
                }
            }
            Node::Variable(v) if v.id == id => return Some(ElementRef::Variable(v)),
            _ => {}
        }
    }
    None
}
fn sections(nodes: &[Node]) -> Vec<&Section> {
    nodes
        .iter()
        .filter_map(|n| {
            if let Node::Section(s) = n {
                Some(s)
            } else {
                None
            }
        })
        .collect()
}
fn display_nodes(nodes: &[Node], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for n in nodes {
        match n {
            Node::Text { value } => f.write_str(value)?,
            Node::Section(s) => display_nodes(&s.children, f)?,
            // Preserve unresolved slots visibly; never silently drop them.
            Node::Variable(v) => write!(f, "{{{{{}}}}}", v.id)?,
        }
    }
    Ok(())
}
impl Document {
    /// Construct in memory without authoring HTMLP. [`crate::lint`] validates it.
    pub fn new(max_tokens: u64, reason: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            version: "0.2".into(),
            tokenizer: "cl100k_base".into(),
            limits: Limits {
                max_tokens: Some(max_tokens),
                per_item: None,
                reason: Some(reason.into()),
            },
            children,
            position: Position::default(),
        }
    }
    pub fn get_element_by_id(&self, id: &str) -> Option<ElementRef<'_>> {
        lookup(&self.children, id)
    }
    pub fn sections(&self) -> Vec<&Section> {
        sections(&self.children)
    }
}
impl Section {
    pub fn new(id: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            id: Some(id.into()),
            limits: Limits::default(),
            children,
            position: Position::default(),
        }
    }
    pub fn get_element_by_id(&self, id: &str) -> Option<ElementRef<'_>> {
        lookup(&self.children, id)
    }
    pub fn sections(&self) -> Vec<&Section> {
        sections(&self.children)
    }
}
impl Node {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text {
            value: value.into(),
        }
    }
}
impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_nodes(&self.children, f)
    }
}
impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        display_nodes(&self.children, f)
    }
}
impl fmt::Display for ElementRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Section(s) => s.fmt(f),
            Self::Variable(v) => write!(f, "{{{{{}}}}}", v.id),
        }
    }
}

/// Per-element measurement. `tokens` is `None` and `deferred` is true when
/// unbound variables prevent an exact count. Render to enforce those budgets.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Measurement {
    pub id: Option<String>,
    pub tokens: Option<u64>,
    pub limit: Option<u64>,
    pub deferred: bool,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "json", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub measurements: Vec<Measurement>,
}
impl Report {
    /// No known errors; deferred measurements still require rendering.
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}
