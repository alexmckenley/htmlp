import { NodeKind, type ContentNode, type Policy, type PromptDocument, type PromptMessage, type Variables } from './types.js';
import { countChars, lint } from './lint.js';
/** Render inert strings; lint failures, missing variables, or oversize values throw. */
export function render(doc: PromptDocument, variables: Variables = {}, policy: Policy = {}): PromptMessage[] {
  const checked = lint(doc, policy);
  if (checked.diagnostics.length) throw new Error(checked.diagnostics.map(d => d.message).join('\n'));
  const expand = (nodes: ContentNode[]): string => nodes.map(n => {
    if (n.kind === NodeKind.Text) return n.value;
    if (n.kind === NodeKind.Section) return expand(n.children);
    if (!Object.hasOwn(variables, n.name) || typeof variables[n.name] !== 'string') throw new Error(`Missing string variable: ${n.name}`);
    const value = variables[n.name]!;
    if (countChars(value) > n.maxChars) throw new Error(`Variable ${n.name} exceeds ${n.maxChars} characters`);
    return value;
  }).join('');
  return doc.children.map(m => ({ role: m.role, content: expand(m.children) }));
}
