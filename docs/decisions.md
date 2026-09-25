# Design decisions and precedents

HTMLP starts with the problem of unbounded agent instructions accumulating in repository rules and skills. The differentiator is enforceable context budgets, especially the total along a directory path.

[Microsoft POML](https://github.com/microsoft/POML) already provides prompt markup, templating, data components, and SDKs. HTMLP does not claim to invent HTML-like prompts. Its initial scope is deliberately narrower: a small content format and monotonic policy enforcement.

[remark](https://github.com/remarkjs/remark) is the architecture reference: parse a document to typed structured data, inspect that representation, and render separately. HTMLP uses [parse5](https://parse5.js.org/) for HTML parsing plus token-level structural validation, rather than regular expressions as a markup parser.

[JSON](https://www.json.org/json-en.html) and [TOML](https://toml.io/en/) are useful examples of compact language documentation. There is no substantiated universal consensus that either has the “best” docs. Our choice is an editorial one: a short start page, a precise standalone specification, runnable examples, and [TypeDoc](https://typedoc.org/) reference pages generated from public types.

- HTML fragment parsing preserves access to ordinary HTML parsers. Explicit end tags avoid HTML's surprising void-element behavior.
- Configuration lives outside documents so content cannot increase its own allowed size.
- Variables declare bounds so static analysis is meaningful before runtime values exist.
- Characters are deterministic across runtimes. Model token limits require an explicit tokenizer and will be a separate contract.
- Types and builders work without authoring HTMLP files. Source extraction will use language-specific compiler integration if added.
- MIT is the initial permissive license. Releases remain alpha until the fixtures and counting rules are stable.
