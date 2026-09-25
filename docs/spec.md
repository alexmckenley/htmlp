# HTMLP 0.2 specification

Status: experimental alpha. Extension: `.htmlp`. Encoding: UTF-8.

## Grammar

A file contains exactly one `htmlp` root. Surrounding whitespace and comments are allowed. Root and section children are text, sections, or variables. Nesting is limited to 64 elements, counting the root. Sources are limited to 4 MiB of UTF-8 bytes before parsing.

```html
<htmlp max-tokens="1k" reason="Shared context.">
<section id="workflow">Make one change. Test it.</section>
</htmlp>
```

Only lowercase `htmlp`, `section`, and `var` are recognized. Start and end tags must match exactly; every element needs an explicit closing tag. Attributes must be quoted, unique, and in the allowed set. Namespaces, self-closing tags, processing instructions, XML declarations, DOCTYPE, and CDATA are errors. No browser repair or external resource resolution occurs.

This is an HTML-compatible fragment vocabulary with stricter XML-style lexical rules, not arbitrary HTML or a complete HTML document. A general HTML parser can read valid HTMLP, but cannot enforce these rules or replace its validator. Markdown is literal text; indentation is not dedented and fenced code does not disable markup parsing. Escape literal `<` and `&`.

## Elements and attributes

| Element | Required | Optional |
| --- | --- | --- |
| `htmlp` | `max-tokens`, `reason` | `version`, `tokenizer`, `max-item-tokens` |
| `section` | none | `id`, `max-tokens`, `max-item-tokens`, `reason` |
| `var` | `id`, `max-tokens`, `reason` | none |

`version` defaults to `0.2`; other versions fail. `tokenizer` defaults to `cl100k_base`. The library permits other identifiers, but linting requires an exactly matching `TokenCounter::name()`. The CLI supports only `cl100k_base`.

A `reason` must contain non-whitespace text whenever either token limit is declared on an element. A single reason explains both limits if both are present. Reasons need not repeat on children that merely inherit a direct-item cap. A reason is excluded from prompt text and appears with budget diagnostics and measurements.

IDs are unique throughout a document and contain one or more ASCII letters, digits, `_`, `-`, `.`, or `:`. Variables and sections share the ID namespace. IDs have no role or trust semantics. For example, `id="system"` is a label your SDK adapter may choose to interpret.

A `var` must be completely empty, including no whitespace, comments, or child elements. Its `id` identifies a required string binding at render time. Extra bindings are ignored. There are no expressions, includes, executable code, or network operations.

## Limits

Unsigned decimal integers denote tokens. A `k` or `K` suffix multiplies by 1,000 and permits up to three fractional digits: `1k`, `1.5k`, `0.001k`. No signs, whitespace, exponent notation, unitless fractions, empty fraction, or fractional tokens are accepted. Values must fit in `u64`. Zero and leading zeros are allowed.

`max-tokens` bounds the whole rendered subtree. The root limit is mandatory. `max-item-tokens` caps each direct child **section**, including that child's full subtree. It does not cap text or variables individually and does not propagate to grandchildren. A child can declare its own item limit. The smaller of the child's own total cap and the parent's item cap wins. For ties the child's own reason is reported.

Each file is checked independently. There is no external configuration, ancestor inheritance, implicit directory-context aggregation, or automatic agent loading. Inline policies and reasons remain editable source; repository review and protected CI must govern policy changes if needed.

## Text and tokens

Rendering concatenates text in source order, strips structural markup and comments, and substitutes variables literally. It inserts no separators. Indentation and whitespace within the root count. CRLF and CR normalize to LF. Text outside the root does not count.

Supported entities are `amp`, `lt`, `gt`, `quot`, `apos`, decimal numeric references, and lowercase-`x` hexadecimal numeric references. Invalid Unicode scalars, disallowed controls, U+FFFE/U+FFFF, and numeric C1 controls U+0080–U+009F fail. The C1 restriction avoids HTML numeric-reference remapping differences. Other named entities are not supported.

Token counters count ordinary text, including special-token-looking strings, without adding protocol markers. No heuristic character-to-token conversion is used. The CLI uses the embedded `cl100k_base` encoding from tiktoken-rs. Counts apply to this encoding only, and exclude provider message framing, tools, or SDK overhead.

Static checking merges adjacent literal text across section boundaries before tokenization. Each unresolved variable reserves its declared allowance and separates literal runs. Checked sums must fit in `u64`. A reservation total is not a mathematical upper bound on the final BPE count: tokenization is not additive. Rendering first requires the static check to pass, then requires every binding, checks each value against its variable bound, and retokenizes every complete rendered subtree. No output is returned on failure. A short runtime value does not rescue an invalid static reservation budget.

## Typed API and JSON

`Document` contains a version, tokenizer identifier, `Limits`, `Vec<Node>`, and source position. `Node` is the enum `Text`, `Section`, or `Variable`. `Section` contains an optional ID, limits, children, and position. `Variable` contains an ID, expanded numeric bound, reason, and position. Constructors can build the same structure without a source file; `lint` validates that structure too.

`get_element_by_id` searches descendants and returns `ElementRef`, a section or variable reference. `sections()` returns direct child sections in source order. `to_string()` extracts text without validation and represents unbound variables as `{{id}}`; it is neither source serialization nor a checked rendered prompt.

The optional `json` feature serializes node variants with a lowercase `kind` discriminator. Sizes are expanded integers. JavaScript consumers need a lossless JSON integer reader for values above `Number.MAX_SAFE_INTEGER`. Source positions use one-based Unicode-scalar line/column and a zero-based UTF-8 byte offset. Zero line/column represents an unavailable location or an in-memory node. The generated [JSON Schema](https://extraloyal.com/htmlp/document.schema.json) describes data shape; semantic invariants still require `lint`.

The `schema` feature generates the document schema from these Rust types. `cargo doc` generates their API reference. Diagnostic codes and JSON fields are experimental in this alpha.

## CLI

`check PATH [--json]` walks files deterministically and checks each `.htmlp` file. `parse FILE` validates syntax and returns an AST; `compile FILE` also checks budgets. `render FILE [--vars FILE]` returns checked plaintext, without adding a newline. Bindings are a JSON object of string values. `watch PATH` polls content every 500 ms and emits editor diagnostics on changes.

Directory walks skip symlinks and directories named `.git`, `node_modules`, `target`, `dist`, and `vendor`; recursion is capped at 128 levels. An explicitly supplied file must have the `.htmlp` extension for `check`. No matching files is an operational error. Other file commands accept an explicit path regardless of extension.

Exit status: 0 success, 1 per-file validation failure (including unreadable input files encountered as file reports), 2 invalid command or operational failure. Diagnostics are `path:line:column: error code: message` on stderr. JSON output and rendered text go to stdout. Watch remains active until interrupted; it is for editors, not a CI success check.
