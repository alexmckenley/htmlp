import { NodeKind, Role, type ContentNode, type DocumentLimits, type PromptDocument, type MessageNode, type SectionNode, type TextNode, type VariableNode } from './types.js';
/** Construct a literal text fragment, usable without parsing a file. */
export function text(value: string): TextNode { return { kind: NodeKind.Text, value }; }
/** Construct a named, bounded runtime string. */
export function variable(name: string, maxChars: number): VariableNode {
  if (!/^[A-Za-z_][A-Za-z0-9_.-]*$/.test(name) || !Number.isSafeInteger(maxChars) || maxChars < 0) throw new Error('Invalid variable name or maxChars');
  return { kind: NodeKind.Variable, name, maxChars };
}
/** Group fragments under a required, document-unique name. */
export function section(name: string, children: ContentNode[], maxChars?: number): SectionNode {
  return { kind: NodeKind.Section, name, children, ...(maxChars === undefined ? {} : { maxChars }) };
}
/** Construct a role-bearing prompt component. */
export function message(role: Role, children: ContentNode[], maxChars?: number): MessageNode {
  return { kind: NodeKind.Message, role, children, ...(maxChars === undefined ? {} : { maxChars }) };
}
export const system = (children: ContentNode[], maxChars?: number): MessageNode => message(Role.System, children, maxChars);
export const user = (children: ContentNode[], maxChars?: number): MessageNode => message(Role.User, children, maxChars);
/** Assemble typed components into the same AST emitted by the parser. */
export function document(children: MessageNode[], limits: DocumentLimits = {}): PromptDocument {
  return { kind: NodeKind.Document, version: '0.1', limits, children };
}
