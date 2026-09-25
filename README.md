# HTMLP

Statically enforceable token limits for context files.

Agents write their own skills. Context fills with slop. Prompts grow unbounded. HTMLP enforces a token budget, pushing agents to decide what belongs, what to cut, and when a larger budget needs justification.

Put a limit and its reason beside the content; check every file before it reaches an agent. [Prompt file guide](https://htmlp.dev/docs/) · [Rust interface guide](https://htmlp.dev/docs/rust/).

```html
<htmlp max-tokens="2k" reason="Loaded on every request.">
<workflow id="routine" max-tokens="500" reason="Keep routine steps short.">
Read the code. Make one change. Run its tests.
</workflow>
</htmlp>
```

Save as `rules.htmlp`. `2k` means 2,000 tokens. Every declared limit requires a nonblank `reason`. Element names are yours to choose; HTMLP validates structure and budgets, not meaning.

## Install and check

Requires a Rust toolchain. This alpha is distributed through GitHub; it is not published to crates.io.

```sh
cargo install --git https://github.com/alexmckenley/htmlp --tag v0.3.0-alpha.1 --features cli
htmlp sign ./prompts
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

## Sign budget changes

Run `htmlp sign ./prompts` after authoring or intentionally changing budgets. It adds a short `sig` beside each declared limit, hashing `max-tokens`, `per-item`, and the decoded `reason`. Content edits do not need re-signing. Equivalent sizes (`1k` and `1000`) have the same signature.

`check`, `compile`, `render`, and `watch` reject missing or stale signatures:

```text
error signature: Missing or mismatched budget signature. If you intend to do this, please rerun `htmlp sign` to regenerate the signature.
```

Review budget changes before re-signing; run only `check` in CI. Signatures are unkeyed checksums, not approvals: an editor can rerun `sign` with the same reason or remove a constraint entirely. Repository review still governs policy changes. Rust callers can explicitly use `Document::sign()`, `sign_source`, or `sign_path`.

## Format

- `<htmlp>` is the single root; `max-tokens` and `reason` are required.
- Every other element name is author-chosen: lowercase ASCII letters, digits, and hyphens, starting with a letter. `<workflow>`, `<system-prompt>`, and `<section>` are all valid and all validated identically.
- Optional `id` supports lookup and must be unique. The name is the element's kind; the ID is its identity. Neither assigns a provider role or trust level.
- Optional `max-tokens` bounds an element's full text, including descendants.
- `per-item` on any element caps each direct child element. For example, `max-tokens="10k" per-item="1k" reason="Keep checks concise."` caps the total at 10,000 tokens and each item at 1,000. A child's own smaller limit still applies.
- `{{question}}` inserts a named string value. Variables have no limits or attributes; repeated names reuse the same binding.
- One `reason` explains the limits declared on that element. Reasons are metadata, excluded from rendered text.

Names are open; the attribute set is closed. Tags are lowercase with quoted attributes, and an unknown attribute is an error anywhere. Use matching end tags, or self-close an empty element: `<notes id="scratch" />`. Escape literal `<` and `&`, even in Markdown fences. Write `&#123;&#123;name}}` to keep a literal `{{name}}`. Whitespace is preserved. Unknown syntax fails instead of being repaired like browser HTML.

Budgets count rendered text with `cl100k_base`, not characters or guessed tokens. The encoding can differ from your model's tokenizer. Static checks defer budgets for subtrees containing variables; `render` checks their final text after substitution. Deferred measurements have `tokens: null` and `deferred: true`. Limits do not include SDK role wrappers or other provider overhead.

## Rust API

The default library uses the allocation-free `xmlparser` tokenizer and RustCrypto `sha2` for budget checksums. HTMLP itself allocates its owned AST. JSON, schema generation, and embedded tokenization are optional features. The CLI includes exact tokenization and consequently vocabulary data.

Markup and Rust build the same tree. `Element` is the primitive in both.

```rust
use htmlp::{Document, Element};

let workflow = Element::new("workflow")
    .id("routine")
    .max_tokens(500, "Keep routine steps short.")
    .template("Read {{path}}. Make one change. Run its tests.")
    .expect("valid template");
let mut document = Document::new(2_000, "Loaded on every request.", vec![workflow.into()]);
document.sign();
```

`.template()` parses `{{name}}` through the same code path as markup text, so this document produces the same nodes as its parsed equivalent, differing only in source positions; `.text()` appends literal content instead. Use `parse`, `parse_file`, `lint`, and `Document::render`. Supply a `TokenCounter`, or enable `tokens` and use `Cl100k`. `elements()` returns direct child elements and `elements_by_name(name)` searches descendants. `to_string()` is an unchecked text view with `{{name}}` placeholders; `render` returns checked text with per-element provenance.

Run the complete file-to-text example with `cargo run --locked --example composition --features tokens`.

Other languages can use the CLI's JSON output and generated schema. Native bindings and source-code literal analysis are future work.

## Structure here, meaning in your types

HTMLP assigns no roles, no message kinds, and no trust levels. An element named `system-prompt` is a label for a human reader and for `elements_by_name`; it never becomes a provider role. Applications that need those distinctions define them in their own type system and pair them with checked output:

```rust
enum Category {
    SystemPrompt,
    ContextBlock { block_id: String },
}
```

HTMLP proves the text is within budget; your types prove the text is attributed. 0.2's optional `runtime` feature encoded one harness's vocabulary — system prompts, assistant history, tool definitions — and is removed in 0.3. See the [Rust interface guide](docs/rust.md) and [design decisions](docs/decisions.md).

## Documentation

[Website and guide](https://htmlp.dev/) · [Specification](docs/spec.md) · [Rust interface](docs/rust.md) · [Generated Rust API](https://htmlp.dev/api/htmlp/index.html) · [JSON Schema](schema/document.schema.json)

[Contributing](CONTRIBUTING.md) · [Design decisions](docs/decisions.md) · [Roadmap](docs/roadmap.md) · [Security](SECURITY.md)

This is a breaking 0.3 alpha; see the [changelog](CHANGELOG.md) to upgrade. The former TypeScript/character-budget implementation remains at [v0.1.0-alpha.1](https://github.com/alexmckenley/htmlp/tree/v0.1.0-alpha.1). Inline constraints are editable: protect budget changes through review or CI policy. A required reason explains a limit; it does not make that limit immutable.

MIT licensed. See [LICENSE](LICENSE).
