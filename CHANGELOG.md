# Changelog

## 0.2.0-alpha.3

- Add `htmlp sign PATH` and short `sig` budget checksums covering `max-tokens`, `per-item`, and `reason`.
- Reject missing and stale signatures in checking, compilation, rendering, and watch diagnostics. Existing files need an explicit initial sign.
- Add `Document::sign()`, `sign_source`, and `sign_path`; retain source formatting during signing.
- Signatures acknowledge edits; they are not authentication or proof of a changed reason.

## 0.2.0-alpha.2

- Rename `max-item-tokens` to `per-item`, and the Rust/JSON field to `per_item`.
- Use `max-tokens="10k" per-item="1k" reason="Keep checks concise."` for a total budget and per-item cap.
- Replace the `var` element with `{{name}}` placeholders; variables have no limits in markup, Rust types, or JSON.
- Defer variable-dependent counts until rendering; report them as `tokens: null`, `deferred: true`.
- Support self-closing empty elements. The previous item-limit attribute is rejected.

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
