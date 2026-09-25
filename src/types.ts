/** Node discriminants are strings in the portable JSON representation. */
export enum NodeKind {
  Document = 'document', Message = 'message', Section = 'section', Text = 'text', Variable = 'variable',
}
/** Roles supported by version 0.1. Provider adaptation belongs to the caller. */
export enum Role { System = 'system', User = 'user' }
/** One-based line/column; zero-based UTF-16 offset, matching JavaScript and parse5. */
export interface Position { line: number; column: number; offset: number }
export interface Located { position?: Position }
export interface TextNode extends Located { kind: NodeKind.Text; value: string }
/** A bounded string substitution; values are never evaluated or parsed as markup. */
export interface VariableNode extends Located { kind: NodeKind.Variable; name: string; maxChars: number }
export interface SectionNode extends Located {
  kind: NodeKind.Section; name: string; maxChars?: number; children: ContentNode[];
}
export type ContentNode = TextNode | VariableNode | SectionNode;
export interface MessageNode extends Located {
  kind: NodeKind.Message; role: Role; maxChars?: number; children: ContentNode[];
}
/** Document-local limits may only tighten the external policy. */
export interface DocumentLimits {
  maxChars?: number; maxSectionChars?: number; maxSystemChars?: number;
  maxUserChars?: number; maxSections?: number;
}
export interface PromptDocument extends Located {
  kind: NodeKind.Document; version: '0.1'; limits: DocumentLimits; children: MessageNode[];
}
/** All character budgets count Unicode code points, NOT model tokens. */
export interface Policy {
  maxFileChars?: number; maxSectionChars?: number; maxSystemChars?: number;
  maxUserChars?: number; maxContextChars?: number; maxSections?: number;
  allowedRoles?: Role[]; requiredSections?: string[]; allowFreeText?: boolean;
}
export interface Diagnostic extends Located { code: string; message: string; severity: 'error'; file?: string }
export interface ParseResult { document?: PromptDocument; diagnostics: Diagnostic[] }
export interface LintResult { diagnostics: Diagnostic[]; maxChars: number }
export interface PromptMessage { role: Role; content: string }
export type Variables = Record<string, string>;
