import { parseFragment, type DefaultTreeAdapterMap } from 'parse5';
import { SAXParser } from 'parse5-sax-parser';
import { NodeKind, Role, type ContentNode, type Diagnostic, type DocumentLimits, type MessageNode, type ParseResult, type Position } from './types.js';
type Node = DefaultTreeAdapterMap['node'];
type Element = DefaultTreeAdapterMap['element'];
const isElement = (node: Node): node is Element => 'tagName' in node;
const location = (node: Node): Position | undefined => {
  const l = node.sourceCodeLocation;
  return l ? { line: l.startLine, column: l.startCol, offset: l.startOffset } : undefined;
};
/** Parse an HTML fragment, rejecting implicit tag repair and unknown syntax. */
export function parse(source: string): ParseResult {
  const diagnostics: Diagnostic[] = [];
  const error = (code: string, message: string, node?: Node) => diagnostics.push({ code, message, severity: 'error', position: node && location(node) });
  // HTML tree construction silently drops some invalid tokens. Validate the token
  // stream too, so a browser's recovery cannot change a prompt's meaning.
  const stack: string[] = [];
  const sax = new SAXParser({ sourceCodeLocationInfo: true });
  sax.on('startTag', token => {
    if (!['htmlp', 'system', 'user', 'section', 'text', 'var'].includes(token.tagName)) error('unknown-element', `Unknown element <${token.tagName}>`);
    if (token.selfClosing) error('explicit-tags', 'Self-closing syntax is not supported; use explicit closing tags');
    stack.push(token.tagName);
    if (stack.length > 68) error('nesting', 'Maximum nesting depth exceeded');
  });
  sax.on('endTag', token => { if (stack.pop() !== token.tagName) error('tag-mismatch', `Unexpected closing tag </${token.tagName}>`); });
  sax.on('doctype', () => error('doctype', 'HTMLP is an HTML fragment; doctype declarations are not supported'));
  sax.end(source);
  if (stack.length) error('explicit-tags', 'All elements require closing tags');
  if (diagnostics.length) return { diagnostics };
  const fragment = parseFragment(source, {
    sourceCodeLocationInfo: true,
    onParseError: e => diagnostics.push({ code: 'html-syntax', message: e.code, severity: 'error', position: { line: e.startLine, column: e.startCol, offset: e.startOffset } }),
  });
  const attributes = (el: Element, allowed: string[]) => {
    const result: Record<string, string> = {};
    for (const a of el.attrs) {
      if (!allowed.includes(a.name)) error('unknown-attribute', `Unknown attribute ${a.name} on <${el.tagName}>`, el);
      result[a.name] = a.value;
    }
    if (!el.sourceCodeLocation?.startTag || !el.sourceCodeLocation?.endTag) error('explicit-tags', `<${el.tagName}> requires explicit opening and closing tags`, el);
    return result;
  };
  const limit = (value: string | undefined, node: Node): number | undefined => {
    if (value === undefined) return undefined;
    const n = Number(value);
    if (!/^\d+$/.test(value) || !Number.isSafeInteger(n)) { error('invalid-limit', 'Limits must be nonnegative safe integers', node); return undefined; }
    return n;
  };
  const meaningful = (nodes: Node[]) => nodes.filter(n => n.nodeName !== '#comment' && !(n.nodeName === '#text' && 'value' in n && !n.value.trim()));
  const roots = meaningful(fragment.childNodes);
  if (roots.length !== 1 || !isElement(roots[0]!) || roots[0].tagName !== 'htmlp') {
    error('root', 'Expected exactly one <htmlp> root; unwrapped content is not a user message');
    return { diagnostics };
  }
  const root = roots[0];
  const rootAttrs = attributes(root, ['version', 'max-chars', 'max-section-chars', 'max-system-chars', 'max-user-chars', 'max-sections']);
  if (rootAttrs.version !== undefined && rootAttrs.version !== '0.1') error('version', 'Only HTMLP version 0.1 is supported', root);
  const limits: DocumentLimits = {};
  for (const [attr, key] of Object.entries({ 'max-chars': 'maxChars', 'max-section-chars': 'maxSectionChars', 'max-system-chars': 'maxSystemChars', 'max-user-chars': 'maxUserChars', 'max-sections': 'maxSections' })) {
    const n = limit(rootAttrs[attr], root); if (n !== undefined) limits[key as keyof DocumentLimits] = n;
  }
  const content = (nodes: Node[], depth = 0): ContentNode[] => {
    if (depth > 64) { error('nesting', 'Maximum nesting depth is 64'); return []; }
    const result: ContentNode[] = [];
    for (const node of nodes) {
      if (node.nodeName === '#comment') continue;
      if (node.nodeName === '#text' && 'value' in node) { result.push({ kind: NodeKind.Text, value: node.value, position: location(node) }); continue; }
      if (!isElement(node)) { error('content', 'Unsupported content', node); continue; }
      if (node.tagName === 'section') {
        const a = attributes(node, ['name', 'max-chars']);
        if (!a.name?.trim()) error('section-name', 'Sections need a nonempty name', node);
        result.push({ kind: NodeKind.Section, name: a.name ?? '', maxChars: limit(a['max-chars'], node), children: content(node.childNodes, depth + 1), position: location(node) });
      } else if (node.tagName === 'var') {
        const a = attributes(node, ['name', 'max-chars']);
        if (!/^[A-Za-z_][A-Za-z0-9_.-]*$/.test(a.name ?? '')) error('variable-name', 'Variables need an identifier name', node);
        const maxChars = limit(a['max-chars'], node);
        if (maxChars === undefined) error('unbounded-variable', 'Every variable needs max-chars for static analysis', node);
        if (node.childNodes.length) error('variable-content', '<var> must be empty', node);
        result.push({ kind: NodeKind.Variable, name: a.name ?? '', maxChars: maxChars ?? 0, position: location(node) });
      } else if (node.tagName === 'text') {
        attributes(node, []);
        if (node.childNodes.some(n => n.nodeName !== '#text' && n.nodeName !== '#comment')) error('text-content', '<text> only accepts literal text', node);
        result.push(...content(node.childNodes, depth + 1));
      } else error('unknown-element', `Unknown element <${node.tagName}>; escape literal markup with &lt;`, node);
    }
    return result;
  };
  const children: MessageNode[] = [];
  for (const node of meaningful(root.childNodes)) {
    if (!isElement(node) || !['system', 'user'].includes(node.tagName)) { error('message', 'The root accepts only <system> and <user> messages', node); continue; }
    const a = attributes(node, ['max-chars']);
    children.push({ kind: NodeKind.Message, role: node.tagName as Role, maxChars: limit(a['max-chars'], node), children: content(node.childNodes), position: location(node) });
  }
  if (!children.length) error('empty-document', 'At least one message is required', root);
  if (diagnostics.length) return { diagnostics };
  return { document: { kind: NodeKind.Document, version: '0.1', limits, children, position: location(root) }, diagnostics };
}
