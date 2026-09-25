export const dynamic = 'force-static';
import Link from 'next/link';
const example = `<htmlp max-chars="4000" max-section-chars="600">
  <system>
    <section name="working-agreement">
      Make the smallest useful change.
      Test the behavior you changed.
    </section>
  </system>
  <user>Review <var name="diff" max-chars="2000"></var></user>
</htmlp>`;
export default function Home() {
  return <main id="main" className="home">
    <div className="eyebrow">HTML FOR PROMPTS · 0.1 ALPHA</div>
    <h1>Kill the<br/>prompt <span>monolith.</span></h1>
    <div className="intro"><p>Your agent’s context deserves boundaries.<br/>Write small, structured prompts. Give every section a budget. Catch the overflow before it reaches your agent.</p><Link className="button" href="/docs/">Read the docs <span aria-hidden="true">↗</span></Link></div>
    <div className="code-window"><div className="code-title"><span>rules.htmlp</span><span>PLAIN TEXT. EXPLICIT LIMITS.</span></div><pre><code>{example}</code></pre><div className="code-footer"><span className="dot"/> Every variable has a bound. Every section has a limit.</div></div>
    <section className="principles" aria-label="How HTMLP works">
      <div><span className="number">01 / AUTHOR</span><h2>Familiar markup.</h2><p>An HTML-compatible fragment with messages, sections, and named variables. Markdown stays plain text inside it.</p></div>
      <div><span className="number">02 / ENFORCE</span><h2>Budgets that stick.</h2><p>Check sections, files, and inherited context. A child directory can tighten a limit. It cannot quietly loosen one.</p></div>
      <div><span className="number">03 / COMPOSE</span><h2>Types, with or without files.</h2><p>Parse into a typed tree, or build the same components in code. Render ordinary messages for your SDK.</p></div>
    </section>
    <div className="closing"><h2>Control your context.<br/>Keep your agents focused.</h2><p>A small, open format. A TypeScript reference implementation.<br/>A portable JSON contract for every other language.</p><a href="https://github.com/alexmckenley/htmlp">Explore the source ↗</a></div>
  </main>;
}
