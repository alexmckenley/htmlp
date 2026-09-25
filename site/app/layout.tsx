import type { Metadata } from 'next';
import Link from 'next/link';
import './globals.css';
import { basePath } from './paths';
export const metadata: Metadata = {
  title: { default: 'HTMLP — Kill the prompt monolith', template: '%s · HTMLP' },
  description: 'HTML-compatible prompts with enforceable context budgets. A small language, typed components, and a linter for agent rules.',
};
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return <html lang="en"><body><a href="#main" className="skip">Skip to content</a><header className="header"><Link href="/" className="brand"><span aria-hidden="true">&lt;/&gt;</span> HTMLP</Link><nav aria-label="Main navigation"><Link href="/docs/">Docs</Link><a href={`${basePath}/api/index.html`}>API</a><a href="https://github.com/alexmckenley/htmlp">GitHub ↗</a></nav></header>{children}<footer><Link href="/" className="brand">HTMLP</Link><span>Small prompts. Explicit contracts.</span><a href="https://github.com/alexmckenley/htmlp/blob/main/LICENSE">MIT licensed</a></footer></body></html>;
}
