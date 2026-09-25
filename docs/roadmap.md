# Roadmap

The 0.2 alpha implements self-contained prompt files, required reasons, exact token budgets, typed Rust nodes, ID lookup, named string variables, a directory checker, editor watch mode, JSON output, and generated API/schema documentation.

Next, validate the format on real prompt files before expanding it. Gather evidence about the need for additional tokenizers and measure parser throughput, binary size, and large-repository watch cost.

Possible later work, not current features:

- C ABI, WebAssembly, or native language bindings.
- Build-tool integrations and static extraction of prompt literals.
- A language server for inline editor diagnostics.
- Explicit composition with aggregate budgets.

Keep versioning deliberate. Do not add arbitrary HTML, executable interpolation, implicit SDK roles, or repository configuration without a concrete use case.
