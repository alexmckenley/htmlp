import { Role, type Policy } from './types.js';
export const DEFAULT_POLICY: Readonly<Required<Policy>> = Object.freeze({
  maxFileChars: 8000, maxSectionChars: 1200, maxSystemChars: 6000,
  maxUserChars: 4000, maxContextChars: 16000, maxSections: 12,
  allowedRoles: [Role.System, Role.User], requiredSections: [], allowFreeText: true,
});
const numeric = ['maxFileChars', 'maxSectionChars', 'maxSystemChars', 'maxUserChars', 'maxContextChars', 'maxSections'] as const;
/** Reject typos and invalid policy values rather than silently ignoring constraints. */
export function validatePolicy(input: unknown): Policy {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new Error('Policy must be an object');
  const p = input as Record<string, unknown>;
  for (const [key, value] of Object.entries(p)) {
    if ((numeric as readonly string[]).includes(key)) {
      if (!Number.isSafeInteger(value) || (value as number) < 0) throw new Error(`${key} must be a nonnegative safe integer`);
    } else if (key === 'allowFreeText') { if (typeof value !== 'boolean') throw new Error('allowFreeText must be boolean'); }
    else if (key === 'allowedRoles') { if (!Array.isArray(value) || value.some(v => !Object.values(Role).includes(v))) throw new Error('allowedRoles must contain system/user roles'); }
    else if (key === 'requiredSections') { if (!Array.isArray(value) || value.some(v => typeof v !== 'string' || !v.trim())) throw new Error('requiredSections must contain nonempty strings'); }
    else throw new Error(`Unknown policy key: ${key}`);
  }
  return p as Policy;
}
/** Combine explicit policies: minima, role intersection, required-section union. */
export function mergePolicies(parent: Policy, child: Policy): Policy {
  validatePolicy(parent); validatePolicy(child);
  const out: Policy = { ...parent, ...child };
  for (const key of numeric) if (parent[key] !== undefined && child[key] !== undefined) out[key] = Math.min(parent[key]!, child[key]!);
  if (parent.allowedRoles && child.allowedRoles) out.allowedRoles = parent.allowedRoles.filter(r => child.allowedRoles!.includes(r));
  out.requiredSections = [...new Set([...(parent.requiredSections ?? []), ...(child.requiredSections ?? [])])];
  if (parent.allowFreeText === false || child.allowFreeText === false) out.allowFreeText = false;
  return out;
}
/** Defaults fill absent values; explicit root policy can choose larger budgets. */
export function resolvePolicy(policy: Policy = {}): Required<Policy> {
  validatePolicy(policy); return { ...DEFAULT_POLICY, ...policy };
}
