//! Strict, self-contained prompt files. The default library depends only on
//! `xmlparser`; token counting is an injected [`TokenCounter`]. Enable `tokens`
//! for CL100K, `json` for serialization, or `cli` for the standalone tool.
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
pub use check::{TokenCounter, lint, render};
pub use files::{FileReport, check_path, parse_file};
pub use model::*;
pub use parser::{parse, parse_limit};

#[cfg(feature = "tokens")]
mod tokenizer;
#[cfg(feature = "tokens")]
pub use tokenizer::Cl100k;
