#!/usr/bin/env node
import { readFile, realpath, stat } from 'node:fs/promises';
import { watch } from 'node:fs';
import path from 'node:path';
import { lint } from './lint.js';
import { render } from './render.js';
import { loadContext, loadPolicy, parseFile, renderContext } from './node.js';
import type { Diagnostic, Variables } from './types.js';
const usage = `HTMLP — Kill the prompt monolith.

htmlp parse FILE [--json]
htmlp lint FILE [--root DIR] [--json]
htmlp watch FILE [--root DIR]
htmlp render FILE [--root DIR] [--vars JSON_FILE]
htmlp context TARGET --root DIR [--json] [--vars JSON_FILE] [--render]

lint discovers .htmlp.json from root (default: cwd) to the file.
context loads rules.htmlp at each level and checks inherited budgets.
Exit codes: 0 success, 1 diagnostics, 2 usage/I/O/configuration error.`;
async function main() {
  const args = process.argv.slice(2);
  if (!args.length || args.includes('--help') || args.includes('-h')) { console.log(usage); return; }
  const command = args.shift(); const target = args.shift();
  if (!target || target.startsWith('-') || !['parse', 'lint', 'watch', 'render', 'context'].includes(command!)) throw new Error(usage);
  let root = process.cwd(); let explicitRoot = false; let json = false; let shouldRender = false; let varsFile: string | undefined;
  while (args.length) {
    const flag = args.shift();
    if (flag === '--json') json = true;
    else if (flag === '--render' && command === 'context') shouldRender = true;
    else if (flag === '--root' || flag === '--vars') {
      const value = args.shift(); if (!value || value.startsWith('--')) throw new Error(`${flag} requires a value`);
      if (flag === '--root') { root = value; explicitRoot = true; } else varsFile = value;
    } else throw new Error(`Unknown argument: ${flag}`);
  }
  if (command === 'context' && !explicitRoot) throw new Error('context requires --root to make the inheritance boundary explicit');
  if (command === 'watch') {
    if (json || varsFile) throw new Error('watch does not accept --json or --vars');
    // Validate the boundary before registering any filesystem watchers.
    await loadPolicy(root, target);
    const file = await realpath(target); const boundary = await realpath(root);
    if (!(await stat(file)).isFile()) throw new Error('watch requires a file');
    let running = false; let pending = false;
    const check = async () => {
      if (running) { pending = true; return; }
      running = true;
      do {
        pending = false;
        console.log('HTMLP checking');
        try {
          const parsed = await parseFile(file);
          const found = parsed.document ? lint(parsed.document, await loadPolicy(root, file)).diagnostics : parsed.diagnostics;
          for (const d of found) console.log(`${file}:${d.position?.line ?? 1}:${d.position?.column ?? 1}: error ${d.code}: ${d.message}`);
        } catch (error) { console.log(`${file}:1:1: error watch: ${String(error).replaceAll('\n', ' ')}`); }
        console.log('HTMLP ready');
      } while (pending);
      running = false;
    };
    let dir = path.dirname(file);
    for (;;) {
      const watchedDir = dir;
      watch(dir, (_event, name) => {
        if (!name || String(name) === '.htmlp.json' || path.join(watchedDir, String(name)) === file) void check();
      }).on('error', error => { console.error(String(error)); process.exitCode = 2; });
      if (dir === boundary) break;
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
    await check();
    return;
  }
  let variables: Variables = {};
  if (varsFile) {
    const parsed: unknown = JSON.parse(await readFile(varsFile, 'utf8'));
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed) || Object.values(parsed).some(v => typeof v !== 'string')) throw new Error('Variables must be a JSON object of strings');
    variables = parsed as Variables;
  }
  let diagnostics: Diagnostic[] = []; let output: unknown;
  if (command === 'context') {
    const result = await loadContext(root, target); diagnostics = result.diagnostics;
    output = shouldRender && !diagnostics.length ? renderContext(result, variables) : result;
  } else {
    const result = await parseFile(target); diagnostics = result.diagnostics;
    output = result.document ?? result;
    if (result.document && command !== 'parse') {
      const policy = await loadPolicy(root, path.resolve(target));
      const checked = lint(result.document, policy); diagnostics = checked.diagnostics.map(d => ({ ...d, file: target }));
      output = command === 'render' && !diagnostics.length ? render(result.document, variables, policy) : { ...checked, diagnostics };
    }
  }
  if (json || command === 'parse' || command === 'render' || shouldRender) console.log(JSON.stringify(output, null, 2));
  else if (diagnostics.length) for (const d of diagnostics) console.error(`${d.file ?? target}:${d.position?.line ?? 1}:${d.position?.column ?? 1}: error ${d.code}: ${d.message}`);
  else console.log(`HTMLP OK: ${target}`);
  if (diagnostics.length) process.exitCode = 1;
}
main().catch(error => { console.error(String(error instanceof Error ? error.message : error)); process.exitCode = 2; });
