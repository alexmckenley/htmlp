# HTMLP 0.1 — draft specification

Status: experimental. HTMLP is an HTML-compatible prompt format with enforceable context budgets. The TypeScript implementation is the reference implementation. This document defines a portable contract; it does not claim native libraries exist for every language.

## Model

A document contains ordered messages. A message has an explicit `system` or `user` role and contains text, named sections, and variables. Sections may nest. There is no implicit role for unwrapped content. HTMLP does not invoke a model, change an agent's trust hierarchy, or guarantee instruction compliance.

The pipeline is `source → parse → typed AST → lint → render → SDK`. Filesystem loading is a separate Node entry point. Constructing an AST in code uses the same linter and renderer as parsing a file.

## Syntax

Files use the `.htmlp` extension and contain exactly one `<htmlp>` root. They are parsed as HTML fragments, not XML. Standard HTML entity decoding, case-insensitive element/attribute names, quoted or unquoted HTML attribute values, and CRLF-to-LF normalization apply. Prefer lowercase names and double-quoted values in authored files.

HTMLP uses application-defined elements including `htmlp`, `system`, `user`, and `text`. An ordinary HTML parser can read them, but they are not all standard HTML vocabulary or standards-conforming Web Component names. “HTML-compatible” means HTML-fragment parsing compatibility, not W3C validation as an ordinary web page.

All elements MUST have explicit opening and closing tags, including `<var></var>`. Self-closing syntax is rejected because HTML does not treat arbitrary elements as void. Mismatched or omitted tags, duplicate attributes, unknown attributes/elements, doctypes, and HTML parser errors are rejected. Maximum content nesting depth is 64. JavaScript, includes, loops, conditionals, and expression evaluation are not part of version 0.1.

```html
<htmlp version="0.1" max-chars="4000" max-section-chars="600">
  <system>
    <section name="working-agreement">Make the smallest useful change.</section>
  </system>
  <user>Review <var name="diff" max-chars="2000"></var></user>
</htmlp>
```

| Element | Permitted attributes | Children |
| --- | --- | --- |
| `htmlp` | `version`, `max-chars`, `max-section-chars`, `max-system-chars`, `max-user-chars`, `max-sections` | One or more `system` / `user` elements |
| `system`, `user` | `max-chars` | Text, `text`, `section`, `var` |
| `section` | Required `name`; optional `max-chars` | Text, `text`, nested `section`, `var` |
| `text` | None | Literal text only |
| `var` | Required `name` and `max-chars` | Empty |

Root `version` defaults to `0.1`. Section names are nonempty, case-sensitive strings, unique throughout a document. Across inherited files, names may repeat. Variable names match `[A-Za-z_][A-Za-z0-9_.-]*` and are case-sensitive. Repeated occurrences reference the same runtime binding, but each occurrence reserves its own budget and checks its own bound.

Free-form text is allowed inside messages by default. Text outside messages is rejected except whitespace between structural elements and HTML comments. `<text>` is an optional explicit wrapper; the AST flattens it into text nodes. Markdown is preserved as ordinary text, not parsed or rendered as Markdown. Escape literal markup as `&lt;` and a literal entity-looking ampersand as `&amp;`.

## Exact counting and rendering

All limits are nonnegative safe integers (0 through 9,007,199,254,740,991). Units are **Unicode code points**, not bytes, UTF-16 code units, grapheme clusters, or tokens. `🦊` counts as 1; `e` followed by a combining accent counts as 2. There is no Unicode normalization.

Text is counted after HTML entity decoding and newline normalization. All whitespace inside messages is preserved and counted, including indentation. Comments and tag/attribute syntax do not reach the prompt and do not consume the character budget. The “file” budget means expanded prompt content, not the on-disk source size.

Sections and `<text>` wrappers add no delimiters or headings. Their child content is concatenated in source order. Authors provide any desired headings, spaces, and newlines themselves. Variables reserve `max-chars` per occurrence during static analysis. This is a worst-case upper bound. A statically oversized template is rejected even if one particular runtime binding would fit.

A section budget includes all descendant text and variable bounds. A message budget includes all its content. File and context budgets sum all message bounds and charge two newline characters between messages (`sum + 2 × max(0, count − 1)`). Role totals apply the same formula to messages of that role. The renderer returns separate messages; these two characters are a documented accounting allowance, not an SDK serialization or token guarantee.

Rendering returns ordered `{ role, content }` objects. No provider-specific message coalescing is performed. Missing variables, non-string values, exceeded variable bounds, and lint errors prevent rendering. Bound values remain literal text and are never reparsed as HTML. This prevents markup expansion, but does not make untrusted instructions safe to place in a system message. Callers remain responsible for provenance and role assignment.

## Policy

`.htmlp.json` is a strict JSON object. Unknown keys and invalid types fail. Root policy values override defaults; thereafter child policies can only tighten the resolved parent policy.

| Property | Default | Meaning |
| --- | ---: | --- |
| `maxFileChars` | 8000 | Maximum expanded content in each file |
| `maxSectionChars` | 1200 | Maximum expanded content in every section |
| `maxSystemChars` | 6000 | System total per file and across inherited context |
| `maxUserChars` | 4000 | User total per file and across inherited context |
| `maxContextChars` | 16000 | Total inherited context along a path |
| `maxSections` | 12 | Number of sections, including nested sections, per file |
| `allowedRoles` | `["system", "user"]` | Allowed message roles |
| `requiredSections` | `[]` | Names required in each rules file at that directory or below |
| `allowFreeText` | `true` | Whether non-whitespace content may appear outside named sections |

Merge numeric limits by minimum, roles by intersection, required sections by union, and `allowFreeText` by logical AND. Empty allowed-role lists allow no messages. Child requests for larger limits have no effect. In-file limits also use the minimum with effective policy; they cannot relax an externally imposed limit.

Inline limits affect their own document. Inherited `.htmlp.json` limits apply to descendants. This separation prevents a child file from increasing its own permitted budget. Repository review/CI must protect the policy itself; the format cannot prevent someone with write access from changing the root policy.

## Directory resolution

The `context` command requires an explicit `--root`. Canonicalize root and target paths. A file target resolves to its containing directory; a directory target uses itself. Reject targets outside the root, including directory symlinks that escape it.

Walk the ancestor chain from root to target inclusive. At each level, merge `.htmlp.json`, then load `rules.htmlp` if present. Do not scan siblings, descend into unrelated directories, or search above the root. Missing optional files are ignored. Invalid policy, read failures, parser errors, and lint violations fail the operation. This loader is for trusted repository content; it is not a sandbox for hostile filesystem trees.

Each file is linted under the policy at its own directory. At the target, the full message sequence is checked against the final context and role budgets and final allowed roles. A child section-size restriction does not retroactively relint ancestor sections. Required sections are per file, not a requirement that an empty directory create a file. A path with no rules yields an empty, valid context. Nothing is automatically loaded by unrelated agents: their harness must call `loadContext`, consume CLI JSON, or use an integration.

## Portable representation

The AST uses `kind` discriminants: `document`, `message`, `section`, `text`, `variable`. Roles are `system` and `user`. Root `version` is `0.1`. Public interfaces and enums live in `src/types.ts`; JSON Schemas in `schema/` and API reference pages are generated from those types. Schema validation checks shape; `lint` additionally checks semantic constraints such as uniqueness and budgets.

Source positions use one-based lines/columns and zero-based UTF-16 offsets. They are optional for programmatically constructed nodes. Parser diagnostics use `severity: "error"`, a stable category `code`, a human-readable `message`, optional `position`, and a `file` when loaded from disk. Some token-structure diagnostics are document-level and have no position in this alpha.

## CLI and integration

- `htmlp parse FILE`: emit the AST as JSON; syntax validation only.
- `htmlp lint FILE [--root DIR] [--json]`: discover policy and report semantic diagnostics.
- `htmlp watch FILE [--root DIR]`: continuously lint saved file and ancestor policy changes; emits editor-compatible diagnostics.
- `htmlp render FILE [--root DIR] [--vars JSON_FILE]`: emit checked messages as JSON.
- `htmlp context TARGET --root DIR [--json]`: load inherited rules, policy, and diagnostics.
- `htmlp context TARGET --root DIR --render [--vars JSON_FILE]`: emit checked inherited messages.

For single-file commands, root defaults to the working directory. Exit status 0 means success, 1 means diagnostics, and 2 means usage/I/O/configuration/runtime binding failure. No network or model calls occur. JSON stdout can be consumed by Python, Rust, Go, Ruby, PHP, or any language with subprocess and JSON support.

## Non-goals of the alpha

No native SDK for every language, editor extension, tokenizer-specific budgets, arbitrary source-code literal extraction, automatic agent installation, imports, execution, or agent framework. These are future proposals, not shipped capabilities. The useful first contract is small: portable prompt structure, monotonic policies, deterministic lint, and safe literal substitution.
