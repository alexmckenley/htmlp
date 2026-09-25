# HTMLP

**Kill the prompt monolith.**

HTML-compatible prompts with enforceable context budgets. Put a limit on every section, every file, and the rules inherited along a directory path.

[Documentation](https://alexmckenley.github.io/htmlp/docs/) · [Specification](docs/spec.md) · [Roadmap](docs/roadmap.md) · [Contributing](CONTRIBUTING.md)

**Experimental 0.1 alpha.** The TypeScript/JavaScript library and CLI work today. JSON output and generated schemas provide a portable contract for other languages. No npm registry release or native SDKs for other languages are published yet.

```html
<htmlp max-chars="4000" max-section-chars="600">
  <system>
    <section name="working-agreement">Make the smallest useful change.
Test the behavior you changed.</section>
  </system>
  <user>Review <var name="diff" max-chars="2000"></var></user>
</htmlp>
```

Prompts tend to grow one pasted instruction at a time. HTMLP turns that growth into a reviewable contract: named sections, explicit roles, bounded variables, and lint errors when content outgrows its budget.

## Try it

Requires Node.js 22.13 or newer for the full project.

```sh
git clone https://github.com/alexmckenley/htmlp.git
cd htmlp
npm ci
npm run build
node dist/cli.js lint examples/rules.htmlp
node dist/cli.js render examples/rules.htmlp --vars examples/variables.json
node dist/cli.js context examples/monorepo/packages/api --root examples/monorepo --json
```

Use `npm link` for a local `htmlp` command, or `npm pack` to produce an installable package. The package name `@htmlp/core` describes the intended API; do not assume it is available from npm.

## Use the types without the language

```ts
import { document, system, section, text, variable, lint, render } from '@htmlp/core';

const prompt = document([
  system([section('task', [text('Review '), variable('diff', 2000)], 2200)]),
]);

const policy = { maxSectionChars: 2200 };
const report = lint(prompt, policy);
const messages = render(prompt, { diff: '+ const answer = 42;' }, policy);
// Pass messages to your SDK's compatible message interface.
```

`parse(source)` returns a typed AST and diagnostics. `parseFile(path)` from `@htmlp/core/node` reads UTF-8 from disk. `loadContext(root, target)` loads inherited rules; `renderContext(context, variables)` produces checked messages. The API reference and JSON Schemas are generated from the public interfaces and enums.

## Monorepo policy

Place `.htmlp.json` and `rules.htmlp` at the repository root and any descendant directories:

```json
{
  "maxFileChars": 4000,
  "maxSectionChars": 600,
  "maxSystemChars": 6000,
  "maxContextChars": 8000,
  "maxSections": 8,
  "allowedRoles": ["system"],
  "requiredSections": ["testing"],
  "allowFreeText": false
}
```

The loader walks root to target. Child policy can tighten limits, never loosen them. Each file is checked locally, then the whole inherited context is checked again. Agents must explicitly integrate the loader or CLI; creating these files does not automatically change another tool's behavior.

## Counting contract

Budgets count Unicode code points in expanded content, **not tokens**. Whitespace inside messages counts; markup and comments do not. Each variable reserves its declared maximum. File and context accounting includes two characters between messages. Runtime substitution is literal, bounded, and never evaluated. See the [specification](docs/spec.md) for exact semantics and defaults.

HTMLP uses HTML fragment parsing with application-defined elements, explicit closing tags, and strict structure. It is HTML-parseable, not a promise of standard HTML vocabulary or arbitrary HTML support.

## Develop

```sh
npm ci
npm run check
npm --prefix site ci
npm run docs:build
npm --prefix site run dev
```

The project includes tests, generated-schema drift checks, continuous integration, GitHub Pages publishing, issue/PR templates, contribution guidance, a security policy, and MIT licensing. API docs are generated with TypeDoc. The site source is in `site/`.

## Prior art

[Microsoft POML](https://github.com/microsoft/POML) already explores prompt markup and templating. HTMLP focuses on budget enforcement and directory policy. [remark](https://github.com/remarkjs/remark) is the parse/typed-tree/tooling reference. Read the [design decisions](docs/decisions.md) for rationale and documentation references.

## License

[MIT](LICENSE). Contributions are welcome; the format is still open to change.
