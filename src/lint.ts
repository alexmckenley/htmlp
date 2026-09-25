import { NodeKind, Role, type ContentNode, type Diagnostic, type Located, type LintResult, type Policy, type PromptDocument } from './types.js';
import { resolvePolicy } from './policy.js';
/** Unicode code points; astral characters count once, combining marks separately. */
export const countChars = (value: string): number => Array.from(value).length;
export const contentMaxChars = (nodes: ContentNode[]): number => nodes.reduce((sum, n) => sum + (n.kind === NodeKind.Text ? countChars(n.value) : n.kind === NodeKind.Variable ? n.maxChars : contentMaxChars(n.children)), 0);
/** Messages are measured as if joined with two newline characters. */
export const joinedSize = (lengths: number[]): number => lengths.reduce((a, b) => a + b, 0) + Math.max(0, lengths.length - 1) * 2;
/** Check worst-case expanded text before runtime substitutions are available. */
export function lint(doc: PromptDocument, policy: Policy = {}): LintResult {
  const p = resolvePolicy(policy);
  const diagnostics: Diagnostic[] = [];
  const error = (code: string, message: string, n: Located = doc) => diagnostics.push({ code, message, severity: 'error', position: n.position });
  const check = (size: number, max: number, label: string, n: Located) => { if (size > max) error('budget-exceeded', `${label}: ${size} exceeds ${max} by ${size - max}`, n); };
  const numericLimit = (value: number | undefined, n: Located) => {
    if (value === undefined) return Infinity;
    if (!Number.isSafeInteger(value) || value < 0) { error('invalid-limit', 'Limits must be nonnegative safe integers', n); return 0; }
    return value;
  };
  for (const value of Object.values(doc.limits)) numericLimit(value, doc);
  if (doc.kind !== NodeKind.Document || doc.version !== '0.1' || !doc.children.length) error('document', 'Expected a nonempty version 0.1 document');
  const sections = new Set<string>(); let sectionCount = 0;
  const walk = (nodes: ContentNode[], inSection: boolean, depth = 0) => {
    if (depth > 64) { error('nesting', 'Maximum nesting depth is 64'); return; }
    for (const n of nodes) {
      if (n.kind === NodeKind.Section) {
        sectionCount++;
        if (!n.name.trim() || sections.has(n.name)) error('section-name', `Section names must be nonempty and unique: ${n.name}`, n);
        sections.add(n.name);
        walk(n.children, true, depth + 1);
        if (!diagnostics.some(d => d.code === 'nesting')) check(contentMaxChars(n.children), Math.min(p.maxSectionChars, numericLimit(doc.limits.maxSectionChars, doc), numericLimit(n.maxChars, n)), `Section "${n.name}"`, n);
      } else {
        if (!p.allowFreeText && !inSection && (n.kind !== NodeKind.Text || n.value.trim())) error('free-text', 'Content must be inside a named section', n);
        if (n.kind === NodeKind.Variable) {
          numericLimit(n.maxChars, n);
          if (!/^[A-Za-z_][A-Za-z0-9_.-]*$/.test(n.name)) error('variable-name', 'Invalid variable identifier', n);
        }
      }
    }
  };
  for (const msg of doc.children) {
    if (!p.allowedRoles.includes(msg.role)) error('role', `Role ${msg.role} is not allowed`, msg);
    walk(msg.children, false);
    if (!diagnostics.some(d => d.code === 'nesting')) check(contentMaxChars(msg.children), numericLimit(msg.maxChars, msg), `${msg.role} message`, msg);
  }
  if (diagnostics.some(d => d.code === 'nesting')) return { diagnostics, maxChars: 0 };
  const maxChars = joinedSize(doc.children.map(m => contentMaxChars(m.children)));
  check(maxChars, Math.min(p.maxFileChars, numericLimit(doc.limits.maxChars, doc)), 'File characters', doc);
  for (const role of Object.values(Role)) {
    const key = role === Role.System ? 'maxSystemChars' : 'maxUserChars';
    check(joinedSize(doc.children.filter(m => m.role === role).map(m => contentMaxChars(m.children))), Math.min(p[key], numericLimit(doc.limits[key], doc)), `${role} characters`, doc);
  }
  check(sectionCount, Math.min(p.maxSections, numericLimit(doc.limits.maxSections, doc)), 'Section count', doc);
  for (const name of p.requiredSections) if (!sections.has(name)) error('required-section', `Missing required section "${name}"`);
  return { diagnostics, maxChars };
}
