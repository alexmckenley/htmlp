# Design decisions

## Small vocabulary, self-contained files

Two elements are sufficient: `htmlp` and `section`. Named string placeholders use `{{name}}`. Variables carry no budgets; checks involving them are deferred until rendering. IDs handle semantic selection without inventing provider-specific roles. Sections can contain Markdown and nested sections. Every limit requires a nearby reason so an editor can understand why it exists. This encourages adherence but does not prevent policy edits.

No top-level configuration or directory inheritance in 0.2. Running on a directory simply checks its files independently. Composition into an agent's context remains the caller's responsibility.

## Strict markup and an existing tokenizer

[xmlparser](https://docs.rs/xmlparser/0.13.6/xmlparser/) has no dependencies and provides a non-allocating XML tokenizer. HTMLP adds nesting, unique-attribute/ID, vocabulary, and budget validation. We do not claim that tokenizer alone validates documents or that HTMLP is a full HTML parser. XML-only features and browser repair behavior are excluded. HTMLP's AST owns strings and does allocate.

## Rust and optional vocabulary data

Rust does not require a garbage collector or language runtime. The library defaults to only the XML tokenizer dependency. Exact token counting uses [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs), an optional feature; the CLI includes it. Encoding tables are substantial data even when the parser is small. Other SDKs can inject a `TokenCounter` matching the document's declared tokenizer.

[Cargo release profiles](https://doc.rust-lang.org/cargo/reference/profiles.html) use size optimization, LTO, one codegen unit, stripping, and abort-on-panic here. These settings favor size, not necessarily maximum throughput. Release artifacts are native binaries and remain subject to platform/ABI requirements. We measure sizes instead of promising a universally tiny executable.

## Documentation as a reference

The homepage and guide are hand-authored HTML with one small stylesheet, no client JavaScript, and no web-font requests. They are readable directly with curl. Rustdoc generates the separate API reference; schemars generates the JSON contract from Rust types.

The compact spec-first approach follows projects such as [JSON](https://www.json.org/json-en.html) and [TOML](https://toml.io/en/), without asserting a universal consensus about the best documentation. Parser-to-typed-tree separation also resembles [mdast](https://github.com/syntax-tree/mdast); HTMLP does not parse Markdown into a Markdown AST.
