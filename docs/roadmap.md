# Roadmap

The 0.3 alpha implements self-contained prompt files with author-chosen element names, optional IDs, required reasons, signed token budgets, typed Rust elements with matching interpolation, ID and name lookup, named string variables, a directory checker, editor watch mode, JSON output, and generated API/schema documentation. Budget counts are exact for the selected tokenizer.

Checked rendering returns immutable text with per-element provenance, including anonymous elements. 0.3 removes the `runtime` feature: typed model requests, roles, and content-source categories belong to the application, not the format.

Next, validate the format on real prompt files before expanding it. Gather evidence about the need for additional tokenizers and measure parser throughput, binary size, and large-repository watch cost.

Possible later work, not current features:

- Element-name conventions published as guidance rather than built-in behavior.
- C ABI, WebAssembly, or native language bindings.
- Build-tool integrations and static extraction of prompt literals.
- A language server for inline editor diagnostics.
- Explicit composition with aggregate budgets.

Keep versioning deliberate. Do not add arbitrary HTML, executable interpolation, a built-in element vocabulary, implicit SDK roles, or repository configuration without a concrete use case.
