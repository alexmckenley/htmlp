import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { loadContext, loadPolicy, renderContext } from '../src/node.js';
const rule = (s: string) => `<htmlp><system><section name="rules">${s}</section></system></htmlp>`;
async function fixture(t: Parameters<Parameters<typeof test>[1]>[0]) {
  const root = await mkdtemp(path.join(tmpdir(), 'htmlp-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(path.join(root, 'a/b'), { recursive: true });
  await writeFile(path.join(root, 'rules.htmlp'), rule('root'));
  await writeFile(path.join(root, 'a/rules.htmlp'), rule('child'));
  return root;
}
test('loads root to leaf including levels without rules; enforces total context', async t => {
  const root = await fixture(t);
  await writeFile(path.join(root, '.htmlp.json'), JSON.stringify({ maxContextChars: 10 }));
  await writeFile(path.join(root, 'a/.htmlp.json'), JSON.stringify({ maxContextChars: 999 }));
  const context = await loadContext(root, path.join(root, 'a/b'));
  assert.equal(context.maxChars, 11); assert.equal(context.policy.maxContextChars, 10);
  assert.equal(context.rules.length, 2); assert.equal(context.diagnostics[0]?.code, 'context-budget');
  assert.throws(() => renderContext(context));
});
test('combined role cap catches individually valid files', async t => {
  const root = await fixture(t);
  await writeFile(path.join(root, '.htmlp.json'), JSON.stringify({ maxSystemChars: 8 }));
  const c = await loadContext(root, path.join(root, 'a'));
  assert.ok(c.diagnostics.some(d => d.message.includes('Inherited system')));
});
test('root policy overrides defaults while descendants only tighten', async t => {
  const root = await fixture(t);
  await writeFile(path.join(root, '.htmlp.json'), JSON.stringify({ maxFileChars: 20000 }));
  await writeFile(path.join(root, 'a/.htmlp.json'), JSON.stringify({ maxFileChars: 30000 }));
  assert.equal((await loadPolicy(root, path.join(root, 'a'))).maxFileChars, 20000);
});
test('context preserves order and accepts a file target', async t => {
  const root = await fixture(t);
  const c = await loadContext(root, path.join(root, 'a/rules.htmlp'));
  assert.deepEqual(renderContext(c).map(m => m.content), ['root', 'child']);
});
test('rejects out-of-root targets and escaping directory symlinks', async t => {
  const root = await fixture(t);
  await assert.rejects(loadContext(root, tmpdir()), /inside root/);
  await symlink(tmpdir(), path.join(root, 'outside'));
  await assert.rejects(loadContext(root, path.join(root, 'outside')), /inside root/);
});
test('invalid config reports error instead of silently dropping it', async t => {
  const root = await fixture(t); await writeFile(path.join(root, 'a/.htmlp.json'), '{bad');
  const c = await loadContext(root, path.join(root, 'a')); assert.equal(c.diagnostics[0]?.code, 'policy');
});
test('CLI JSON AST and status codes are usable from any language', async t => {
  const root = await fixture(t); const file = path.join(root, 'rules.htmlp');
  const ast = JSON.parse(execFileSync(process.execPath, ['dist/cli.js', 'parse', file], { encoding: 'utf8' }));
  assert.equal(ast.kind, 'document');
  await writeFile(path.join(root, '.htmlp.json'), JSON.stringify({ maxFileChars: 1 }));
  const bad = spawnSync(process.execPath, ['dist/cli.js', 'lint', file, '--root', root, '--json'], { encoding: 'utf8' });
  assert.equal(bad.status, 1); assert.equal(JSON.parse(bad.stdout).diagnostics[0].code, 'budget-exceeded');
  const usage = spawnSync(process.execPath, ['dist/cli.js', 'context', root], { encoding: 'utf8' });
  assert.equal(usage.status, 2);
});
