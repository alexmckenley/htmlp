import { access, readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
for (const file of ['index.html', 'docs/index.html', 'api/index.html', 'schema/document.schema.json', 'schema/policy.schema.json', 'spec.md', 'favicon.svg']) {
  await access(`dist/client/${file}`);
}
const prefix = process.env.HTMLP_BASE_PATH || '';
for (const [file, title] of [['index.html', 'Kill the'], ['docs/index.html', 'A boundary for every prompt.']]) {
  const html = await readFile(`dist/client/${file}`, 'utf8');
  assert.ok(html.includes(title), `${file} is not the intended route`);
  assert.ok(html.includes(`${prefix}/api/index.html`), `${file} has an incorrect API link`);
  for (const match of html.matchAll(/(?:src|href)="([^"]+)"/g)) {
    const url = match[1];
    if (url.startsWith('/') && !url.startsWith('//')) assert.ok(url.startsWith(`${prefix}/`), `Unprefixed URL: ${url}`);
    if (url.startsWith(`${prefix}/assets/`)) await access(`dist/client${url.slice(prefix.length).split('?')[0]}`);
  }
}
console.log('Static homepage, guide, API, schema, and asset paths verified.');
