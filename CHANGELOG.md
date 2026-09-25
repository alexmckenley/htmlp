# Changelog

## 0.2.0-alpha.1

Breaking Rust rewrite of the experimental format.

- Self-contained `.htmlp` files with `max-tokens` and `max-item-tokens`.
- Required nonblank `reason` on every declared budget, including variables.
- Exact `cl100k_base` counts in the CLI; injectable tokenizer in the Rust library.
- Three elements: `htmlp`, `section`, and `var`; strict markup and literal Markdown.
- Typed nodes, ID lookup, JSON AST, generated schema and Rust API reference.
- Directory checking, editor watch mode, and checked literal interpolation.
- Plain HTML homepage and documentation.

Removes the 0.1 TypeScript package, character budgets, external policy files, directory inheritance, and provider-specific message builders. Old source and artifacts remain at the 0.1 release tag. There is no automatic migration.

## 0.1.0-alpha.1

Initial experimental TypeScript implementation. Superseded by 0.2.
