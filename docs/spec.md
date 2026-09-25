# HTMLP 0.2 specification

Status: experimental alpha. Extension: `.htmlp`. Encoding: UTF-8.

## Grammar

A file contains exactly one `htmlp` root. Surrounding whitespace and comments are allowed. Root and section children are text, sections, or variables. Nesting is limited to 64 elements, counting the root. Sources are limited to 4 MiB of UTF-8 bytes before parsing.

```html
<htmlp max-tokens="1k" reason="Shared context.">
<section id="workflow">Make one change. Test it.</section>
</htmlp>
```

Only lowercase `htmlp` and `section` elements are recognized. Start and end tags must match exactly; empty elements may self-close, such as `<section id="notes" />`. Attributes must be quoted, unique, and in the allowed set. Namespaces, processing instructions, XML declarations, DOCTYPE, and CDATA are errors. No browser repair or external resource resolution occurs.

This is HTML-inspired markup with strict XML-style lexical rules, not arbitrary HTML or a complete HTML document. A general HTML parser cannot replace its validator; in particular, browser HTML does not honor self-closing syntax on non-void elements. Markdown is literal text; indentation is not dedented and fenced code does not disable markup parsing. Escape literal `<` and `&`.

## Elements and attributes

| Element | Required | Optional |
| --- | --- | --- |
| `htmlp` | `max-tokens`, `reason` | `version`, `tokenizer`, `per-item`, `sig` |
| `section` | none | `id`, `max-tokens`, `per-item`, `reason`, `sig` |

`version` defaults to `0.2`; other versions fail. `tokenizer` defaults to `cl100k_base`. The library permits other identifiers, but linting requires an exactly matching `TokenCounter::name()`. The CLI supports only `cl100k_base`.

A `reason` must contain non-whitespace text whenever either token limit is declared on an element. A single reason explains both limits if both are present. Reasons need not repeat on children that merely inherit a direct-item cap. A reason is excluded from prompt text and appears with budget diagnostics and measurements.

Section IDs must be unique and contain one or more ASCII letters, digits, `_`, `-`, `.`, or `:`. IDs have no role or trust semantics. For example, `id="system"` is a label your SDK adapter may choose to interpret.

## Variables

Write `{{name}}` in text to insert a string binding. Names use the same characters as section IDs, with no surrounding whitespace. Repeated references reuse the same binding, but variable names cannot collide with section IDs. Placeholders have no attributes, token limits, or reasons; these fields are also absent from the Rust `Variable` type and rejected by JSON deserialization. The old `var` element is not supported.

Placeholders are recognized in raw text, including Markdown code fences, before entity decoding. Write `&#123;&#123;name}}` for literal `{{name}}`. Placeholders are not expanded in attributes or comments and cannot span markup or comments. An opening `{{` without a closing `}}`, an empty name, or an invalid name is an error. Closing braces without an opening placeholder remain literal text, allowing ordinary JSON.

Every referenced name needs a string binding at render time. Extra bindings are ignored. Values are inserted literally and are never reparsed as markup or placeholders. There are no expressions, includes, executable code, or network operations.

## Limits

Unsigned decimal integers denote tokens. A `k` or `K` suffix multiplies by 1,000 and permits up to three fractional digits: `1k`, `1.5k`, `0.001k`. No signs, whitespace, exponent notation, unitless fractions, empty fraction, or fractional tokens are accepted. Values must fit in `u64`. Zero and leading zeros are allowed.

`max-tokens` bounds the whole rendered subtree. The root limit is mandatory. `per-item` caps each direct child **section**, including that child's full subtree. It does not cap text or variables individually and does not propagate to grandchildren. A child can declare its own item limit. The smaller of the child's own total cap and the parent's item cap wins. For ties the child's own reason is reported.

Each file is checked independently. There is no external configuration, ancestor inheritance, implicit directory-context aggregation, or automatic agent loading. Inline policies and reasons remain editable source; repository review and protected CI must govern policy changes if needed.

## Budget signatures

Every element declaring `max-tokens` or `per-item` requires a matching `sig` at check, compile, and render time. Unsigned source may be authored and parsed; `sign` generates the attribute. A signature on an element without a local limit is an error. Signatures cover only the two local limits and their reason, not prompt content, IDs, tokenizer, hierarchy, inherited limits, or author identity. Variables have no signatures.

The canonical byte sequence is:

1. ASCII `htmlp-budget-v1` followed by one zero byte.
2. For `max-tokens`, then `per-item`: one byte `0` if absent, or one byte `1` followed by the expanded unsigned integer as eight big-endian bytes.
3. The decoded, line-ending-normalized reason's UTF-8 byte length as eight big-endian bytes, followed by those UTF-8 bytes. No trimming or Unicode normalization occurs.

`sig` is the first eight bytes of SHA-256 of this sequence, written as 16 lowercase hexadecimal characters. Attribute order, quote style, numeric entity spelling, and equivalent token units do not change it. Test vector: `max-tokens="1k" per-item="100" reason="Shared &amp; brief."` produces `29580b676d56bf50`.

Missing or mismatched signatures produce diagnostic code `signature` with guidance to rerun `htmlp sign`. Signing replaces stale values, adds missing ones, and removes orphan signatures. It preserves other source bytes, including Markdown, comments, attribute spelling, and line endings. All selected files are parsed before any writes; each changed file is replaced atomically with its permissions retained. A later I/O failure can leave earlier files updated; this is not a directory transaction. Avoid concurrent edits while signing. Signed output must still fit the 4 MiB source cap.

This is an explicit change-acknowledgment step, not cryptographic authentication. Anyone able to edit the source can re-sign unchanged reasons or remove entire constraints. It does not prove a reason changed or that a person approved it. Do not automatically sign in CI; check committed signatures and review budget changes.

## Text and tokens

Rendering concatenates text in source order, strips structural markup and comments, and substitutes variables literally. It inserts no separators. Indentation and whitespace within the root count. CRLF and CR normalize to LF. Text outside the root does not count.

Supported entities are `amp`, `lt`, `gt`, `quot`, `apos`, decimal numeric references, and lowercase-`x` hexadecimal numeric references. Invalid Unicode scalars, disallowed controls, U+FFFE/U+FFFF, and numeric C1 controls U+0080–U+009F fail. The C1 restriction avoids HTML numeric-reference remapping differences. Other named entities are not supported.

Token counters count ordinary text, including special-token-looking strings, without adding protocol markers. No heuristic character-to-token conversion is used. The CLI uses the embedded `cl100k_base` encoding from tiktoken-rs. Counts apply to this encoding only, and exclude provider message framing, tools, or SDK overhead.

Static checking merges adjacent literal text across section boundaries before tokenization. A subtree containing any unresolved variable has no known token count: its measurement is `tokens: null`, `deferred: true`. No per-variable allowance or partial token estimate is used. Other, fully static subtrees still have their budgets checked. A successful static check therefore does not certify variable-dependent budgets.

Rendering requires every binding, substitutes literal values, and tokenizes every complete rendered subtree. File, section, and direct-item budgets are enforced against this final text; variables have no separate cap. No output is returned on failure. This also handles tokenization changes at interpolation boundaries, where counts are not additive.

## Typed API and JSON

`Document` contains a version, tokenizer identifier, `Limits`, `Vec<Node>`, and source position. `Node` is the enum `Text`, `Section`, or `Variable`. `Section` contains an optional ID, limits, children, and position. `Variable` contains only an ID and source position. Constructors can build the same structure without a source file; `Document::sign()` explicitly accepts its budgets; `lint` validates that structure and its signatures too.

`get_element_by_id` searches descendants and returns `ElementRef`, a section or variable reference (the first occurrence for repeated variable names). `sections()` returns direct child sections in source order. `to_string()` extracts text without validation and represents unbound variables as `{{id}}`; it is neither source serialization nor a checked rendered prompt.

The optional `json` feature serializes node variants with a lowercase `kind` discriminator. Sizes are expanded integers. JavaScript consumers need a lossless JSON integer reader for values above `Number.MAX_SAFE_INTEGER`. Source positions use one-based Unicode-scalar line/column and a zero-based UTF-8 byte offset. Zero line/column represents an unavailable location or an in-memory node. The generated [JSON Schema](https://htmlp.dev/document.schema.json) describes data shape; semantic invariants still require `lint`.

The `schema` feature generates the document schema from these Rust types. `cargo doc` generates their API reference. Diagnostic codes and JSON fields are experimental in this alpha.

## CLI

`sign PATH` updates budget signatures. `check PATH [--json]` walks files deterministically and checks each `.htmlp` file. `parse FILE` validates syntax and returns an AST; `compile FILE` also checks fully static budgets; variable-dependent budgets still require rendering. `render FILE [--vars FILE]` returns checked plaintext, without adding a newline. Bindings are a JSON object of string values. `watch PATH` polls content every 500 ms and emits editor diagnostics on changes.

Directory walks skip symlinks and directories named `.git`, `node_modules`, `target`, `dist`, and `vendor`; recursion is capped at 128 levels. An explicitly supplied file must have the `.htmlp` extension for `check` and `sign`. No matching files is an operational error. Other file commands accept an explicit path regardless of extension.

Exit status: 0 success, 1 per-file validation failure (including unreadable input files encountered as file reports), 2 invalid command or operational failure (including signing preflight failure). Diagnostics are `path:line:column: error code: message` on stderr. JSON output and rendered text go to stdout. Successful `check` output reports how many budgets are deferred. Watch remains active until interrupted; it is for editors, not a CI success check.
