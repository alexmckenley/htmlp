# Runtime prompts

Enable the `runtime` feature to construct attributed model requests without writing an HTMLP file. It enables JSON types, but not vocabulary tables, networking, an async runtime, or a provider SDK.

```rust
use htmlp::runtime::{ContentSource, ModelRequest, PromptFragment, Role};

let request = ModelRequest::builder("my-model")
    .fragment(PromptFragment::text(
        ContentSource::SystemPrompt,
        Role::System,
        "Review the change.",
    ))
    .build();
let estimate = request.estimate();
assert!(estimate.tokens_for(&ContentSource::SystemPrompt) > 0);
```

## Attribution

`PromptFragment` requires a `ContentSource`, `Role`, and typed content. Its fields are private, and its byte count is derived from immutable content. `ModelRequestBuilder` accepts attributed fragments and typed tool definitions. There is no raw-string request input or untagged fragment constructor. This guarantees attribution is supplied; it cannot verify a caller chose the correct source.

`ContentSource` distinguishes system prompts, named context blocks, tool definitions, user messages, session messages, assistant history, named tool outputs, named reminders, named hooks, and compaction summaries. Source attribution and message role are independent. A context block may be sent with a user role. These categories are Rust/JSON types; category attributes are not yet part of HTMLP markup.

`ContentBlock` represents text, reasoning, images, tool uses, and tool results. `ToolUseId` is a string-backed identity supplied by the caller; HTMLP does not generate IDs. Tool schemas use JSON. Provider adapters choose their own wire framing, reasoning support, cache behavior, and role mapping.

## Checked file rendering

`render_checked(&document, &bindings, &counter)` returns an immutable `RenderedPrompt` only after validating signatures, bindings, and every file/section/item budget. It exposes:

- `text()` and consuming `into_text()`.
- `measurement()`: the complete text's tokenizer identifier and token count.
- `report()`: per-section budget measurements; these overlap and must not be summed.
- `section(id)`: a view of a named section from the already checked whole document. Parent and sibling constraints cannot be skipped by selecting a section.
- `spans()`: non-overlapping rendered UTF-8 byte ranges, section ancestry, and optional variable names. These partition the text without double-counting nested sections.

Use `PromptFragment::checked(source, role, &rendered)` to construct an attributed text fragment from the result. The file's IDs never choose SDK roles or source categories implicitly. Converting checked output into ordinary text and modifying it does not preserve its validation guarantee.

## Estimates and reported usage

These measurements serve different purposes:

| Type | Meaning |
| --- | --- |
| `TextMeasurement` | Tokenizer-specific count of the complete rendered document |
| `RequestEstimate` | Heuristic allocation across request sources, including image reserves |
| `ReportedUsage` | Provider-reported totals for one call |
| `TokenUsage` | Optional estimate, optional reported usage, and context window |

The default `ByteEstimator` aggregates UTF-8 payload bytes by the full typed source, then rounds each source up at four bytes per token. Images contribute an 8,192-token reserve instead of base64 transport bytes. Both values are explicit policy fields; the divisor must be nonzero. `ModelRequest::estimate()` uses this policy. `RequestEstimate::total_tokens()` derives its result from the source entries, so there is no separately mutable total. Empty payloads cost zero; nonempty text never vanishes through rounding.

These counts exclude provider framing. Images, tool schemas, and provider reasoning can be transformed or omitted by an adapter. Independent text token counts also do not generally add up across concatenation boundaries. Do not treat a category estimate as an exact or provider-reported token count. `bytes_by_source()` remains a raw payload byte measurement, including image transport data.

`TokenUsage.estimate: None` means attribution is unavailable, as with an external harness that constructs its own prompt. It does not mean every category used zero tokens. `prompt_tokens()` prefers reported prompt fill and otherwise uses the estimate; it returns `None` if neither exists.

`ReportedUsage.input_tokens` is uncached input. Cache reads and writes are separate input lanes; `prompt_tokens()` adds these three lanes. Reasoning tokens are already part of output, and must never be added to output again. Provider adapters normalize their source-specific conventions.

`merge_snapshot()` takes field-wise maxima of cumulative, partially populated snapshots from the same call. `add_call()` sums distinct completed calls. These operations are deliberately separate to avoid double-counting stream updates or dropping repeated requests.

Generated contracts: [model request](../schema/request.schema.json), [token usage](../schema/usage.schema.json). Generate Rust API documentation with `cargo doc --all-features --no-deps`.
