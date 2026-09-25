# What to do next

## First: validate on actual prompt sprawl

Convert one real rules tree, preserving the exact intended prompt text. Measure the largest sections, each file, and the composed context for representative directories. Choose budgets from that inventory, then enforce them in CI. Record where named sections improve review and where the syntax gets in the way.

## Stabilize the contract

Gather feedback on explicit end tags, whitespace counting, section naming, and the split between file-local and repository policy. Expand cross-language conformance fixtures. Lock the 0.1 specification before promising compatibility.

## Add adoption tools in this order

1. Editor diagnostics and HTMLP file association using the existing parser/linter.
2. Real agent harness adapters that load rules explicitly and preserve provenance.
3. Tokenizer plugins with named model/tokenizer versions and measured limits.
4. Native SDKs driven by the JSON schema and shared conformance fixtures.
5. A TypeScript build plugin that extracts supported literal constructors into compiled artifacts; reject dynamic expressions rather than pretending to analyze them.

No schedule or support commitment is implied. Propose changes through issues and small pull requests. Avoid adding a general template programming language until concrete adoption requires it.
