# Rust interface

HTMLP is one primitive in two syntaxes. Markup and Rust build the same tree: an `Element` with a name, an optional `id`, optional budgets, and children. Nothing in the library assigns provider roles, message kinds, or trust levels; your application decides what a name means.

```toml
[dependencies]
htmlp = { git = "https://github.com/alexmckenley/htmlp", tag = "v0.3.0-alpha.1" }
```

This alpha is distributed through GitHub, not crates.io. Add `tokens` for the built-in `Cl100k` tokenizer, `json` for serialization. In a source checkout, run `cargo run --locked --example composition --features tokens`.

## Build a document

```rust
use htmlp::{Document, Element};

let workflow = Element::new("workflow")
    .id("routine")
    .max_tokens(500, "Keep routine steps short.")
    .text("Read the code. Make one change. Run its tests.");
let mut document = Document::new(2_000, "Loaded on every request.", vec![workflow.into()]);
document.sign(); // Explicitly accept the budgets authored above.
```

`Element::new(name)` takes any valid element name: lowercase ASCII letters, digits, and hyphens, starting with a letter. `htmlp` is reserved for the root. The name is the element's kind; the optional `id` is its identity. `max_tokens` and `per_item` each require a reason and clear any existing signature, so changing a budget in Rust also requires `Document::sign()`.

Builder methods append in call order: `.text(value)` adds literal text, `.variable(name)` adds a placeholder, `.element(child)` nests, and `.template(source)` parses `{{name}}` interpolation. `Node::text`, `Node::variable`, and `From<Element> for Node` construct nodes directly.

## Interpolation

`.template()` accepts the same syntax as markup text and produces the same nodes, so a Rust-built document and a parsed one differ only in source positions:

```rust
use htmlp::{Bindings, Cl100k, Document, Element, parse};

let answer = Element::new("answer")
    .template("Answer: {{question}}")
    .expect("valid template");
let mut built = Document::new(1_000, "Request context.", vec![answer.into()]);
built.sign();
let parsed = parse(
    r#"<htmlp max-tokens="1k" reason="Request context." sig="a66b5750507d50df"><answer>Answer: {{question}}</answer></htmlp>"#,
)
.expect("valid markup");

let counter = Cl100k::new().expect("tokenizer");
let bindings = Bindings::from([("question".into(), "What changed?".into())]);
assert_eq!(
    built.render(&bindings, &counter).expect("valid").text(),
    parsed.render(&bindings, &counter).expect("valid").text(),
);
```

Only `{{name}}` is special. Other characters, including `<` and `&`, are literal — `.template()` does not decode entities, because entity syntax belongs to the file format. Use `.text()` for content containing literal braces, and `parse_template(source)` when you want the nodes without an element wrapper. Template errors carry line, column, and byte offset like parse errors.

## Render and check

```rust
use htmlp::{Bindings, Cl100k, parse_file};

let document = parse_file("rules.htmlp").expect("valid markup");
let counter = Cl100k::new().expect("tokenizer");
let bindings = Bindings::from([("question".into(), "What changed?".into())]);
let rendered = document
    .render(&bindings, &counter)
    .expect("valid signatures, bindings, and budgets");
```

`Document::render` is the checked entry point; `render_checked(&document, &bindings, &counter)` is the same function as a free function. It returns a `RenderedDocument` only after validating element names, unique IDs, budget signatures, every binding, and every root, element, and per-item budget. There is no constructor or deserializer for checked output, and no partial result on failure.

`parse` and `parse_file` validate syntax only. `lint(&document, &counter)` checks the structure and its fully static budgets; `render` also binds variables and enforces final budgets. `to_string()` on a `Document`, `Element`, or `ElementRef` is an unchecked text view that shows unbound variables as `{{name}}`.

## Read checked output

| Method | Returns |
| --- | --- |
| `text()`, `into_text()` | The final text, borrowed or owned |
| `measurement()` | `TextMeasurement`: tokenizer identifier and total count |
| `element(id)` | One element's text, selected only after the whole document passed |
| `elements_by_name(name)` | Every element of that name, in source order |
| `elements()` | Every `RenderedElement`: origin and byte range, including empty ones |
| `spans()` | Non-overlapping `RenderedSpan` ranges with element ancestry |
| `report()` | Per-element budget `Measurement`s |

`ElementOrigin` carries `name`, `path`, `id`, and `position`. `path` is the chain of child indices from the document root, so anonymous elements are still identifiable and two elements sharing a name never collapse. Selecting one element cannot skip its parents' constraints, because selection happens after the whole document validates.

`elements()` ranges nest and may overlap; `spans()` partitions the text exactly once. Budget `Measurement`s include descendants, so summing a parent and its children double-counts. Group by `spans()` when attributing tokens to categories.

## Naming and application types

Names and IDs are labels. HTMLP has no built-in `system-prompt`, `user-message`, or `tool-definition` element, and it will not derive one from a name you choose. An application that needs those distinctions should define them in its own type system and attach them to rendered output:

```rust
enum Category {
    SystemPrompt,
    ContextBlock { block_id: String },
}
```

Then pair a `Category` with `rendered.element("routine")` or with whole-document `text()`. This keeps two guarantees separate: HTMLP proves the text is within budget; your types prove the text is attributed. A document whose root element is named `system-prompt` still carries no role — the name documents intent for a human reader and for `elements_by_name` lookup.

## Tokenizers

`TokenCounter` is a trait with `name()` and `count(text)`. `lint` requires the counter's name to match the document's declared `tokenizer`, which defaults to `cl100k_base`. Enable `tokens` for `Cl100k`, or implement the trait against your provider's tokenizer. Counts exclude provider message framing, tool schemas, and SDK overhead, and text token counts are not additive across concatenation boundaries — which is why `render` re-tokenizes each complete subtree after substitution.

## Reference

- [Generated Rust API](https://htmlp.dev/api/htmlp/index.html)
- [Document JSON Schema](../schema/document.schema.json)
- [Specification](spec.md)
- [Runnable example](../examples/composition.rs)

Generate API documentation with `cargo doc --all-features --no-deps`. The `json` feature serializes documents, measurements, and rendered provenance; the `schema` feature generates the document schema from the same types. Other languages can use the CLI's JSON output; native bindings are future work.
