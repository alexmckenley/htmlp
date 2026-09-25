# Changelog

## 0.3.0-alpha.1

Breaking. Element names are author-chosen and the runtime API is gone.

- Replace the single `section` element with author-chosen names: lowercase ASCII letters, digits, and hyphens, starting with a letter. `htmlp` stays reserved for the root, and the attribute set stays closed.
- Rename the Rust `Section` type to `Element` and give it a `name`; `id` is optional. `Node::Section` becomes `Node::Element`; `sections()` becomes `elements()`, plus `elements_by_name(name)`.
- Add an `Element` builder: `new`, `id`, `text`, `variable`, `template`, `element`, `max_tokens`, and `per_item`. Setting a budget clears its signature.
- Add `parse_template` and `Element::template` so Rust interpolation parses `{{name}}` through the same code path as markup text. An in-memory document produces the same nodes as its parsed equivalent, differing only in source positions.
- Add `Document::render(&bindings, &counter)` as the checked entry point, `Document::element`, and `Document::elements_by_name`.
- Rename `RenderedPrompt` to `RenderedDocument` and `SectionOrigin` to `ElementOrigin`; add `name` and a `path` of child indices so anonymous elements stay identifiable. Add `RenderedElement`, `elements()`, and `elements_by_name()`.
- Add `name` and `path` to `Measurement`; budget diagnostics label anonymous elements by name and path.
- Remove the `runtime` feature: `ContentSource`, `Role`, `PromptFragment`, `ContentBlock`, `ToolDefinition`, `ModelRequest`, `ByteEstimator`, `RequestEstimate`, `ReportedUsage`, and `TokenUsage`. Roles, message kinds, and usage accounting belong to the application; HTMLP validates structure and budgets. Move these types into your own crate, then pair a category with `Document::render` output.
- Remove the request and usage JSON schemas and the `runtime` examples; add `examples/composition.rs`.
- Documents declare `version` `0.3`; `0.2` files fail. Existing files upgrade by renaming `section` tags to whatever name fits, or leaving them as `section`, which remains a valid name.

## 0.2.0-alpha.4

- Add optional `htmlp::runtime`: attributed prompt fragments, roles, tool/image/reasoning blocks, model requests, and source categories.
- Separate categorized request estimates from provider-reported usage; represent unavailable attribution explicitly.
- Unify source rounding and image reserves in `ByteEstimator`; add distinct stream-snapshot merging and per-call aggregation.
- Add immutable `render_checked` output with tokenizer measurements, checked section selection, and non-overlapping provenance spans.
- Generate request and usage JSON schemas alongside the file AST schema.

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
