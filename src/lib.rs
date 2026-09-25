//! Strict, self-contained prompt files. The default library uses `xmlparser` and `sha2`; token counting is an injected [`TokenCounter`]. Enable `tokens`
//! for CL100K, `json` for serialization, or `cli` for the standalone tool.
//! Enable `runtime` for attributed model requests and token usage accounting,
//! independently of prompt files or provider SDKs. Use [`render_checked`] to
//! obtain immutable, validated text and section provenance before constructing
//! a runtime fragment. See the [Rust guide](https://htmlp.dev/docs/runtime/).
//!
//! ```
//! let doc = htmlp::parse(r#"<htmlp max-tokens="10k" reason="Shared context."><section id="task">Review.</section></htmlp>"#)?;
//! assert_eq!(doc.get_element_by_id("task").unwrap().to_string(), "Review.");
//! # Ok::<(), htmlp::Diagnostic>(())
//! ```
#![forbid(unsafe_code)]
mod check;
mod files;
mod model;
mod parser;
mod rendered;
pub use rendered::{RenderedPrompt, RenderedSpan, SectionOrigin, TextMeasurement, render_checked};
#[cfg(feature = "runtime")]
pub mod runtime;
mod signing;
pub use check::{TokenCounter, lint, render};
pub use files::{FileReport, check_path, parse_file, sign_path};
pub use model::*;
pub use parser::{parse, parse_limit};
pub use signing::{budget_signature, sign_source};

#[cfg(feature = "tokens")]
mod tokenizer;
#[cfg(feature = "tokens")]
pub use tokenizer::Cl100k;
