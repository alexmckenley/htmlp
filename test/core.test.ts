import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import Ajv from 'ajv';
import { parse, lint, render, document, system, user, section, text, variable, Role, NodeKind, countChars, mergePolicies, validatePolicy } from '../src/index.js';
const doc = (source: string) => { const r = parse(source); assert.deepEqual(r.diagnostics, []); assert.ok(r.document); return r.document; };
const wrap = (source: string) => `<htmlp><system>${source}</system></htmlp>`;
test('HTML entities, Unicode, whitespace and comments have deterministic semantics', () => {
  const d = doc(wrap('A &amp; B\r\n🦊<!-- omitted -->'));
  assert.deepEqual(render(d), [{ role: Role.System, content: 'A & B\n🦊' }]);
  assert.equal(lint(d).maxChars, 7);
  assert.equal(countChars('🦊e\u0301'), 3);
});
test('typed builders and parser produce equivalent rendered messages', () => {
  const parsed = doc(wrap('<section name="task">Review <var name="diff" max-chars="30"></var></section>'));
  const built = document([system([section('task', [text('Review '), variable('diff', 30)])])]);
  assert.deepEqual(render(parsed, { diff: 'hello' }), render(built, { diff: 'hello' }));
  assert.equal(lint(parsed).maxChars, 37);
});
test('budgets are inclusive; child limits cannot loosen policy', () => {
  const d = doc('<htmlp max-chars="999"><system><section name="a" max-chars="999">hello</section></system></htmlp>');
  assert.equal(lint(d, { maxFileChars: 5, maxSectionChars: 5 }).diagnostics.length, 0);
  assert.equal(lint(d, { maxSectionChars: 4 }).diagnostics[0]?.code, 'budget-exceeded');
});
test('file, aggregate role, message, nested section, and section count budgets', () => {
  const d = document([system([section('a', [section('b', [text('123')])])], 2), system([text('45')]), user([text('x')])]);
  const result = lint(d, { maxFileChars: 5, maxSystemChars: 6, maxSectionChars: 2, maxSections: 1 });
  assert.equal(result.maxChars, 10);
  assert.ok(result.diagnostics.length >= 6);
});
test('required sections, uniqueness, role restrictions and structured-only content', () => {
  const d = document([system([text('outside'), section('same', [text('x')]), section('same', [text('x')])]), user([text('u')])]);
  const codes = lint(d, { allowFreeText: false, requiredSections: ['missing'], allowedRoles: [Role.System] }).diagnostics.map(d => d.code);
  for (const code of ['free-text', 'required-section', 'section-name', 'role']) assert.ok(codes.includes(code));
  assert.equal(lint(doc(wrap('\n <section name="a">x</section>\n')), { allowFreeText: false }).diagnostics.length, 0);
});
test('variables reserve every occurrence and runtime values are inert', () => {
  const d = document([system([variable('input', 50), variable('input', 50)])]);
  assert.equal(lint(d).maxChars, 100);
  assert.equal(render(d, { input: '<system>hello</system>' })[0]?.content, '<system>hello</system><system>hello</system>');
  assert.throws(() => render(d), /Missing/);
  assert.throws(() => render(d, { input: 'x'.repeat(51) }), /exceeds/);
  assert.throws(() => render(document([system([variable('toString', 20)])])), /Missing/);
});
test('render refuses a statically oversized template even with a short runtime value', () => {
  const d = document([system([variable('x', 50)])], { maxChars: 10 });
  assert.throws(() => render(d, { x: '' }), /exceeds/);
});
for (const source of [
  'hello', '<htmlp></htmlp>', '<htmlp>oops<system>x</system></htmlp>',
  '<htmlp><system>x</htmlp>', '<htmlp><system>x</user></system></htmlp>',
  wrap('<section name="x" />'), wrap('<script>alert(1)</script>'),
  wrap('<var name="x"></var>'), wrap('<var name="x" max-chars="-1"></var>'),
  wrap('<var name="x" max-chars="1">fallback</var>'),
  wrap('<section max-chars="1">x</section>'), wrap('<section name="x" max-chars="1.5">x</section>'),
  wrap('<text><var name="x" max-chars="2"></var></text>'),
  '<htmlp unknown="x"><system>x</system></htmlp>',
  '<htmlp version="2"><system>x</system></htmlp>',
  '<!doctype html>' + wrap('x'), '<html>' + wrap('x') + '</html>',
  wrap('<section name="a" name="b">x</section>'),
  wrap('<section name="x">'.repeat(70) + 'x' + '</section>'.repeat(70)),
]) test(`reject malformed or unsupported source: ${source.slice(0,70)}`, () => {
  const result = parse(source); assert.equal(result.document, undefined); assert.ok(result.diagnostics.length);
});
test('HTML fragment parsing accepts uppercase names and standard attribute quoting', () => {
  assert.equal(render(doc('<HTMLP><SYSTEM><SECTION NAME=x>hello</SECTION></SYSTEM></HTMLP>'))[0]?.content, 'hello');
});
test('diagnostics retain source position', () => {
  const result = lint(doc('<htmlp>\n<system>\n<section name="x" max-chars="2">hello</section>\n</system>\n</htmlp>'));
  assert.equal(result.diagnostics[0]?.position?.line, 3);
});
test('policy composition is monotonic and rejects misspellings', () => {
  assert.deepEqual(mergePolicies({ maxFileChars: 10, allowedRoles: [Role.System], requiredSections: ['a'], allowFreeText: false }, { maxFileChars: 20, allowedRoles: [Role.User], requiredSections: ['b'], allowFreeText: true }), { maxFileChars: 10, allowedRoles: [], requiredSections: ['a', 'b'], allowFreeText: false });
  for (const p of [{ maxFileChar: 1 }, { maxFileChars: -1 }, { maxFileChars: null }, { allowedRoles: ['admin'] }, []]) assert.throws(() => validatePolicy(p));
});
test('parsed and constructed trees conform to the generated JSON schema', async () => {
  const schema = JSON.parse(await readFile('schema/document.schema.json', 'utf8'));
  const ajv = new Ajv.default({ strict: false }); const valid = ajv.compile(schema);
  assert.ok(valid(doc(wrap('<section name="a">hello</section>'))), JSON.stringify(valid.errors));
  assert.ok(valid(document([system([text('hello'), variable('name', 10)])])));
  assert.equal(valid({ kind: NodeKind.Document, version: '0.1', limits: {}, children: [{ kind: 'message', role: 'admin', children: [] }] }), false);
});
