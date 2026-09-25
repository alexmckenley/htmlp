import { readFile, realpath, stat } from 'node:fs/promises';
import path from 'node:path';
import { parse } from './parser.js';
import { lint, contentMaxChars, joinedSize } from './lint.js';
import { mergePolicies, resolvePolicy, validatePolicy } from './policy.js';
import { render } from './render.js';
import { Role, type Diagnostic, type ParseResult, type Policy, type PromptDocument, type PromptMessage, type Variables } from './types.js';

/** Read UTF-8 source from the filesystem; I/O errors remain ordinary exceptions. */
export async function parseFile(file: string): Promise<ParseResult> {
  const result = parse(await readFile(file, 'utf8'));
  result.diagnostics = result.diagnostics.map(d => ({ ...d, file }));
  return result;
}
export interface LoadedRule { file: string; document: PromptDocument; policy: Policy }
export interface ContextResult {
  rules: LoadedRule[]; policy: Policy; diagnostics: Diagnostic[]; maxChars: number;
}
async function optionalRead(file: string): Promise<string | undefined> {
  try { return await readFile(file, 'utf8'); }
  catch (error) { if ((error as NodeJS.ErrnoException).code === 'ENOENT') return undefined; throw error; }
}
/** Resolve canonical root-to-target directories. Never search above the explicit root. */
async function directories(root: string, target: string): Promise<string[]> {
  const base = await realpath(root);
  const canonicalTarget = await realpath(target);
  const end = (await stat(canonicalTarget)).isDirectory() ? canonicalTarget : path.dirname(canonicalTarget);
  const relative = path.relative(base, end);
  if (relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) throw new Error('Target must be inside root');
  const result = [base]; let current = base;
  for (const part of relative.split(path.sep).filter(Boolean)) { current = path.join(current, part); result.push(current); }
  return result;
}
/** Load ancestor policy files, enforcing monotonic constraints. */
export async function loadPolicy(root: string, target: string): Promise<Policy> {
  let policy: Policy | undefined;
  for (const dir of await directories(root, target)) {
    const file = path.join(dir, '.htmlp.json');
    const source = await optionalRead(file);
    const local = source === undefined ? {} : validatePolicy(JSON.parse(source));
    policy = policy === undefined ? resolvePolicy(local) : mergePolicies(policy, local);
  }
  return policy!;
}
/** Load rules.htmlp at every directory level, root first; also lint aggregate budgets. */
export async function loadContext(root: string, target: string): Promise<ContextResult> {
  const rules: LoadedRule[] = []; const diagnostics: Diagnostic[] = [];
  let policy: Policy | undefined;
  for (const dir of await directories(root, target)) {
    const config = path.join(dir, '.htmlp.json');
    try {
      const source = await optionalRead(config);
      const local = source === undefined ? {} : validatePolicy(JSON.parse(source));
      policy = policy === undefined ? resolvePolicy(local) : mergePolicies(policy, local);
    } catch (error) {
      diagnostics.push({ code: 'policy', message: String(error), severity: 'error', file: config });
      return { rules, policy: policy ?? resolvePolicy(), diagnostics, maxChars: 0 };
    }
    const file = path.join(dir, 'rules.htmlp');
    const source = await optionalRead(file);
    if (source === undefined) continue;
    const result = parse(source);
    diagnostics.push(...result.diagnostics.map(d => ({ ...d, file })));
    if (result.document) {
      const checked = lint(result.document, policy);
      diagnostics.push(...checked.diagnostics.map(d => ({ ...d, file })));
      rules.push({ file, document: result.document, policy });
    }
  }
  const effective = resolvePolicy(policy);
  const messages = rules.flatMap(r => r.document.children);
  const maxChars = joinedSize(messages.map(m => contentMaxChars(m.children)));
  const check = (size: number, limit: number, label: string) => { if (size > limit) diagnostics.push({ code: 'context-budget', message: `${label}: ${size} characters exceeds ${limit} by ${size - limit}`, severity: 'error', file: target }); };
  check(maxChars, effective.maxContextChars, 'Inherited context');
  for (const role of Object.values(Role)) {
    check(joinedSize(messages.filter(m => m.role === role).map(m => contentMaxChars(m.children))), effective[role === Role.System ? 'maxSystemChars' : 'maxUserChars'], `Inherited ${role} context`);
  }
  // A target policy also restricts which ancestor roles may reach the agent.
  for (const m of messages) if (!effective.allowedRoles.includes(m.role)) diagnostics.push({ code: 'context-role', message: `Inherited role ${m.role} is not allowed at target`, severity: 'error', file: target });
  return { rules, policy: effective, diagnostics, maxChars };
}
/** Render a checked context without reordering or silently merging provider messages. */
export function renderContext(context: ContextResult, variables: Variables = {}): PromptMessage[] {
  if (context.diagnostics.length) throw new Error(context.diagnostics.map(d => d.message).join('\n'));
  return context.rules.flatMap(rule => render(rule.document, variables, rule.policy));
}
