export const dynamic = 'force-static';
import type { Metadata } from 'next';
import { basePath } from '../paths';
export const metadata: Metadata = { title: 'Documentation' };
const code = `<htmlp max-chars="4000" max-section-chars="600">
  <system>
    <section name="working-agreement">Make the smallest useful change.
Test the behavior you changed.</section>
  </system>
  <user>Review <var name="diff" max-chars="2000"></var></user>
</htmlp>`;
const setup = `git clone https://github.com/alexmckenley/htmlp.git
cd htmlp
npm ci
npm run build
node dist/cli.js lint examples/rules.htmlp
node dist/cli.js render examples/rules.htmlp --vars examples/variables.json`;
const builder = `import {
  document, system, section, text, variable, lint, render,
} from '@htmlp/core';

const prompt = document([
  system([section('task', [text('Review '), variable('diff', 2000)])]),
]);
const policy = { maxSectionChars: 2200 };
const report = lint(prompt, policy);
const messages = render(prompt, { diff: '+ const x = 1;' }, policy);`;
export default function Docs() {
  return <main id="main" className="docs">
    <div className="eyebrow">THE HTMLP GUIDE / 0.1 ALPHA</div><h1>A boundary for every prompt.</h1>
    <p className="lead">Author familiar markup. Parse a typed tree. Enforce a budget before any text reaches your agent.</p>
    <nav aria-label="Guide contents"><a href="#start">Start</a><a href="#language">Language</a><a href="#budgets">Budgets</a><a href="#inheritance">Inheritance</a><a href="#types">Typed API</a><a href="#reference">Reference</a></nav>
    <p className="note">HTMLP is experimental. The TypeScript library and CLI are implemented. Install from source or a local package; an npm registry release has not been published.</p>
    <h2 id="start">Start with one file</h2><p>Use Node.js 22.13 or later. Clone the project, build the CLI, and try a checked example.</p><pre><code>{setup}</code></pre>
    <p>For a local <code>htmlp</code> command, run <code>npm link</code>. To install the library into another project, run <code>npm pack</code> and install the resulting archive there.</p>
    <h3>Lint while editing</h3><p>After building, open an HTMLP file in VS Code and run <strong>Tasks: Run Task → HTMLP: watch current file</strong>. Saved content and ancestor policy changes update the Problems panel. Stop the task when switching files. Other editors can consume <code>htmlp watch FILE --root DIR</code> diagnostics.</p>
    <h2 id="language">Six elements. Explicit roles.</h2><pre><code>{code}</code></pre>
    <div className="table-wrap"><table><thead><tr><th>Element</th><th>Purpose</th><th>Attributes</th></tr></thead><tbody>
      <tr><td><code>htmlp</code></td><td>One document root</td><td><code>version</code>, file/role/section limits</td></tr>
      <tr><td><code>system</code></td><td>A system message</td><td><code>max-chars</code></td></tr>
      <tr><td><code>user</code></td><td>A user message</td><td><code>max-chars</code></td></tr>
      <tr><td><code>section</code></td><td>A named group; may nest</td><td>Required <code>name</code>, optional <code>max-chars</code></td></tr>
      <tr><td><code>text</code></td><td>Optional literal-text wrapper</td><td>None</td></tr>
      <tr><td><code>var</code></td><td>A runtime string binding</td><td>Required <code>name</code> and <code>max-chars</code></td></tr>
    </tbody></table></div>
    <p>The root accepts explicit messages only. Inside messages, write free-form text or named sections. Section names must be unique within the document. Markdown inside a message remains plain text.</p>
    <p>Always close tags, including <code>&lt;var&gt;&lt;/var&gt;</code>. Escape literal markup with <code>&amp;lt;</code>. Unknown elements, unknown attributes, malformed nesting, and unbounded variables are errors. No scripts or expressions execute.</p>
    <p>HTMLP is an HTML-parseable fragment with application-defined elements. It does not claim that every tag belongs to standard HTML vocabulary. A generic HTML parser can read the tree; HTMLP adds the semantic checks.</p>
    <h2 id="budgets">Count what reaches the prompt</h2><p>A limit counts Unicode code points in expanded text. Tags and comments do not count. Whitespace inside messages does, including indentation. Variables reserve their declared maximum at every occurrence. File and context totals also charge two characters between messages.</p>
    <p>Limits are inclusive: 600 characters fits a 600-character budget. A nested section counts toward its parent’s budget. Section names do not become headings; write headings and separators in the text when you want them in the prompt.</p>
    <p className="note">Characters are not tokens. This contract is deterministic across languages. Tokenizer-specific limits are a future extension, not an estimate disguised as a guarantee.</p>
    <h3>Use repository policy to enforce limits</h3><pre><code>{`{
  "maxFileChars": 4000,
  "maxSectionChars": 600,
  "maxSystemChars": 6000,
  "maxContextChars": 8000,
  "maxSections": 8,
  "allowedRoles": ["system"],
  "requiredSections": ["testing"],
  "allowFreeText": false
}`}</code></pre>
    <p>Save this as <code>.htmlp.json</code>. Root configuration chooses the policy; child configuration can only tighten it. Numeric limits take the minimum, allowed roles intersect, required sections accumulate, and disabling free text stays disabled. An in-file attribute cannot increase a policy limit.</p>
    <p>Without configuration, defaults are 8,000 characters per file, 1,200 per section, 6,000 system characters, 4,000 user characters, 16,000 inherited context characters, and 12 sections. Both roles and free text are allowed; no section names are required.</p>
    <h2 id="inheritance">Small files can still make a giant prompt</h2><pre><code>{`repo/
├── .htmlp.json
├── rules.htmlp
└── packages/
    └── api/
        ├── .htmlp.json
        └── rules.htmlp

htmlp context packages/api --root . --json
htmlp context packages/api --root . --render`}</code></pre>
    <p>The loader walks from the explicit root to the target, merging policy and loading <code>rules.htmlp</code> at every level. It checks each file and the combined context. It never scans sibling directories or searches above the root.</p>
    <p>Per-file checks use the policy where that file lives. Aggregate context and role budgets use the target’s effective policy. Required sections apply to each rules file, not to directories without a file.</p>
    <p>Your agent must call the loader or consume the CLI output. HTMLP does not automatically modify another tool’s rules discovery.</p>
    <h2 id="types">Use the components without files</h2><pre><code>{builder}</code></pre>
    <p>The constructors produce the same discriminated tree as the parser. <code>NodeKind</code> and <code>Role</code> are exported enums. The linter checks either source, and the renderer returns ordered <code>{'{ role, content }'}</code> messages for a compatible SDK.</p>
    <pre><code>{`import { parseFile, loadContext, renderContext } from '@htmlp/core/node';

const parsed = await parseFile('rules.htmlp');
const context = await loadContext(process.cwd(), 'packages/api');
const messages = renderContext(context);`}</code></pre>
    <p>Every runtime variable must be a string within its bound. Values are inserted literally and never reparsed as markup. Missing bindings and lint failures stop rendering. Keep untrusted input in the appropriate role; literal insertion alone does not prevent prompt injection.</p>
    <h3>Any language can consume the contract</h3><pre><code>{`import json, subprocess

ast = json.loads(subprocess.check_output([
    "node", "dist/cli.js", "parse", "examples/rules.htmlp"
], text=True))`}</code></pre>
    <p>The CLI emits JSON, and the AST has a generated JSON Schema. Native SDKs beyond TypeScript are future work. CLI exit codes are 0 for success, 1 for diagnostics, and 2 for usage, I/O, or binding errors.</p>
    <h2 id="reference">Reference, generated from the source</h2>
    <ul><li><a href={`${basePath}/api/index.html`}>Typed API reference</a> — interfaces, enums, constructors, and functions.</li><li><a href={`${basePath}/schema/document.schema.json`}>AST JSON Schema</a> and <a href={`${basePath}/schema/policy.schema.json`}>policy JSON Schema</a>.</li><li><a href={`${basePath}/spec.md`}>Standalone language specification</a> — exact syntax, counting, and resolution rules.</li><li><a href="https://github.com/alexmckenley/htmlp/tree/main/examples">Runnable examples</a> and <a href="https://github.com/alexmckenley/htmlp/blob/main/docs/roadmap.md">what comes next</a>.</li></ul>
    <p>HTMLP builds on the typed-tree approach of <a href="https://github.com/remarkjs/remark">remark</a>. <a href="https://github.com/microsoft/POML">Microsoft POML</a> is prior art for prompt markup. HTMLP’s focus is enforceable context budgets and inherited policy.</p>
  </main>;
}
