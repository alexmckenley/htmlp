# HTMLP

A strict markup format for prompt files with explicit token limits.

Keep shared instructions within a defined budget. Put a limit and its reason beside the content; check every file before it reaches an agent.

```html
<htmlp max-tokens="2k" reason="Loaded on every request.">
<section id="workflow" max-tokens="500" reason="Keep routine steps short.">
Read the code. Make one change. Run its tests.
</section>
</htmlp>
```

Save as `rules.htmlp`. `2k` means 2,000 tokens. Every declared limit requires a nonblank `reason`.

## Install and check

Requires a Rust toolchain. This alpha is distributed through GitHub; it is not published to crates.io.

```sh
cargo install --git https://github.com/alexmckenley/htmlp --tag v0.2.0-alpha.2 --features cli
htmlp check ./prompts
```

Or clone the repository and run `cargo install --path . --features cli`.

The CLI recursively checks `.htmlp` files independently. No repository configuration, ancestor discovery, or implicit prompt loading. `htmlp watch ./prompts` reports changes for editor integration.

```sh
htmlp check rules.htmlp --json   # measurements and diagnostics
htmlp parse rules.htmlp         # syntax-checked JSON AST
htmlp compile rules.htmlp       # also enforce budgets
htmlp render request.htmlp --vars values.json
```

## Format

- `<htmlp>` is the single root; `max-tokens` and `reason` are required.
- `<section>` groups Markdown. Optional `id` supports lookup. Optional `max-tokens` bounds its full text, including descendants.
- `per-item` on a root or section caps each direct child section. For example, `max-tokens="10k" per-item="1k" reason="Keep checks concise."` caps the total at 10,000 tokens and each item at 1,000. A child's own smaller limit still applies.
- `{{question}}` inserts a named string value. Variables have no limits or attributes; repeated names reuse the same binding.
- One `reason` explains the limits declared on that element. Reasons are metadata, excluded from rendered text.

Tags are lowercase with quoted attributes. Use matching end tags, or self-close an empty element: `<section id="notes" />`. Escape literal `<` and `&`, even in Markdown fences. Write `&#123;&#123;name}}` to keep a literal `{{name}}`. Whitespace is preserved. Unknown syntax fails instead of being repaired like browser HTML.

Budgets count rendered text with `cl100k_base`, not characters or guessed tokens. The encoding can differ from your model's tokenizer. Static checks defer budgets for subtrees containing variables; `render` checks their final text after substitution. Deferred measurements have `tokens: null` and `deferred: true`. Limits do not include SDK role wrappers or other provider overhead.

## Rust API

The default library has one dependency: the allocation-free `xmlparser` tokenizer. HTMLP itself allocates its owned AST. JSON, schema generation, and embedded tokenization are optional features. The CLI includes exact tokenization and consequently vocabulary data.

```rust
use htmlp::{Document, Node, Section};

let document = Document::new(1000, "Shared context.", vec![
    Node::Section(Section::new("system", vec![Node::text("Review the change.")])),
]);
assert_eq!(document.get_element_by_id("system").unwrap().to_string(), "Review the change.");
```

Use `parse`, `parse_file`, `lint`, and `render`. Supply a `TokenCounter`, or enable `tokens` and use `Cl100k`. `sections()` returns direct child sections. `to_string()` is an unchecked text view with `{{name}}` placeholders; `render` returns checked substitutions. Section IDs do not assign SDK message roles.

Other languages can use the CLI's JSON output and generated schema. Native bindings and source-code literal analysis are future work.

## Documentation

[Website and guide](https://extraloyal.com/htmlp/) · [Specification](docs/spec.md) · [Generated Rust API](https://extraloyal.com/htmlp/api/htmlp/index.html) · [JSON Schema](schema/document.schema.json)

[Contributing](CONTRIBUTING.md) · [Design decisions](docs/decisions.md) · [Roadmap](docs/roadmap.md) · [Security](SECURITY.md)

This is a breaking 0.2 alpha. The former TypeScript/character-budget implementation remains at [v0.1.0-alpha.1](https://github.com/alexmckenley/htmlp/tree/v0.1.0-alpha.1). Inline constraints are editable: protect budget changes through review or CI policy. A required reason explains a limit; it does not make that limit immutable.

MIT licensed. See [LICENSE](LICENSE).
