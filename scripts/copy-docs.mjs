import { cp, mkdir } from 'node:fs/promises';
await mkdir('site/public/schema', { recursive: true });
await cp('schema', 'site/public/schema', { recursive: true });
await cp('docs/spec.md', 'site/public/spec.md');
if (process.env.HTMLP_BASE_PATH) {
  const { readdir, readFile, writeFile } = await import('node:fs/promises');
  const prefix = process.env.HTMLP_BASE_PATH;
  async function rewrite(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const file = `${dir}/${entry.name}`;
      if (entry.isDirectory()) await rewrite(file);
      else if (entry.name.endsWith('.html')) {
        const source = await readFile(file, 'utf8');
        await writeFile(file, source.replaceAll('href="/"', `href="${prefix}/"`).replaceAll('href="/docs/"', `href="${prefix}/docs/"`));
      }
    }
  }
  await rewrite('site/public/api');
}
