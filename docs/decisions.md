# Design decisions

## One primitive, no vocabulary

An element is the only grouping. Its name is author-chosen, its ID is optional, and neither means anything to HTMLP. `<system-prompt>` and `<workflow>` are validated identically. Earlier alphas recognized only `section`, which pushed every distinction into IDs and invited a built-in vocabulary of roles; open names keep the labels where an author already reads them while leaving interpretation to the caller.

Names are open, but the attribute set is closed: an unknown attribute is an error on every element. So a new name is free and cannot introduce new behavior. Nesting, unique IDs, budgets, and reasons are still validated the same way.

Markup and Rust build the same tree. `Element::new(name)` mirrors a start tag, and `.template()` parses `{{name}}` with the same code path as markup text, so an in-memory document produces the same nodes as a parsed one, differing only in source positions. Having two syntaxes disagree about what a document is would make the format the real API and the Rust types a lossy mirror.

Named string placeholders use `{{name}}`. Variables carry no budgets; checks involving them are deferred until rendering. Every limit requires a nearby reason so an editor can understand why it exists. This encourages adherence but does not prevent policy edits.

No top-level configuration or directory inheritance in 0.3. Running on a directory simply checks its files independently. Composition into an agent's context remains the caller's responsibility.

## Strict markup and an existing tokenizer

[xmlparser](https://docs.rs/xmlparser/0.13.6/xmlparser/) has no dependencies and provides a non-allocating XML tokenizer. HTMLP adds nesting, unique-attribute/ID, vocabulary, and budget validation. We do not claim that tokenizer alone validates documents or that HTMLP is a full HTML parser. XML-only features and browser repair behavior are excluded. HTMLP's AST owns strings and does allocate.

## Rust and optional vocabulary data

Rust does not require a garbage collector or language runtime. The library defaults to the XML tokenizer and RustCrypto SHA-256 dependencies. Exact token counting uses [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs), an optional feature; the CLI includes it. Encoding tables are substantial data even when the parser is small. Other SDKs can inject a `TokenCounter` matching the document's declared tokenizer.

[Cargo release profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) use size optimization, LTO, one codegen unit, stripping, and abort-on-panic here. These settings favor size, not necessarily maximum throughput. Release artifacts are native binaries and remain subject to platform/ABI requirements. We measure sizes instead of promising a universally tiny executable.

## Documentation as a reference

The homepage and guide are hand-authored HTML with one small stylesheet, no client JavaScript, and no web-font requests. They are readable directly with curl. Rustdoc generates the separate API reference; schemars generates the JSON contract from Rust types.

The compact spec-first approach follows projects such as [JSON](https://www.json.org/json-en.html) and [TOML](https://toml.io/en/), without asserting a universal consensus about the best documentation. Parser-to-typed-tree separation also resembles [mdast](https://github.com/syntax-tree/mdast); HTMLP does not parse Markdown into a Markdown AST.

## Explicit budget acknowledgment

A 16-character `sig` covers normalized local limits and their reason. `sign` updates it; checking never does. This catches accidental policy edits without changing prompt content. SHA-256 comes from RustCrypto rather than a custom hash implementation. The truncated, unkeyed checksum is not an authorization mechanism; repository review remains necessary for policy changes.

## Structure here, meaning in the application

0.2 shipped an optional `runtime` feature with typed model requests, roles, and content-source categories. Those types encoded one harness's idea of a prompt — system prompts, assistant history, tool definitions — which made HTMLP look like a provider SDK and made its vocabulary a public contract. 0.3 removes them. HTMLP validates structure and budgets; an application defines what its content means.

The boundary is deliberate. HTMLP owns names, optional IDs, budgets with reasons and signatures, variables, exact tokenizer measurement, and immutable checked output with per-element provenance. Roles, message kinds, tool-call identity, request settings, cache behavior, provider framing, and usage accounting belong to the caller, who can express them in types that fit its own harness. Sunflower, the first consumer, keeps exactly those types and gained the freedom to change them without a format release.

What remains is the bridge: `Document::render` returns checked text with an `ElementOrigin` per element, and the caller pairs that with its own category. Element names document intent for a human reader and for `elements_by_name`; they never become a role. See the [Rust guide](rust.md).
