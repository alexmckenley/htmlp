//! Strict, self-contained prompt files. The default library uses `xmlparser` and `sha2`; token counting is an injected [`TokenCounter`]. Enable `tokens`
//! for CL100K, `json` for serialization, or `cli` for the standalone tool.
//! Markup and the Rust API share one primitive: an [`Element`] with a name, an
//! optional id, and budgets. Use [`Document::render`] to obtain immutable,
//! validated text with per-element provenance. See the
//! [Rust guide](https://htmlp.dev/docs/rust/).
//!
//! ```
//! let doc = htmlp::parse(r#"<htmlp max-tokens="10k" reason="Shared context."><task id="review">Review.</task></htmlp>"#)?;
//! assert_eq!(doc.get_element_by_id("review").unwrap().to_string(), "Review.");
//! # Ok::<(), htmlp::Diagnostic>(())
//! ```
#![forbid(unsafe_code)]
mod check;
mod files;
mod model;
mod parser;
mod rendered;
pub use rendered::{
    ElementOrigin, RenderedDocument, RenderedElement, RenderedSpan, TextMeasurement, render_checked,
};
mod signing;
pub use check::{TokenCounter, lint, render};
pub use files::{FileReport, check_path, parse_file, sign_path};
pub use model::*;
pub use parser::{parse, parse_limit, parse_template};
pub use signing::{budget_signature, sign_source};

#[cfg(feature = "tokens")]
mod tokenizer;
#[cfg(feature = "tokens")]
pub use tokenizer::Cl100k;
