import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
const require = createRequire(import.meta.url);
const TJS = require('typescript-json-schema');
const program = TJS.getProgramFromFiles(['src/types.ts'], { strictNullChecks: true });
await mkdir('schema', { recursive: true });
for (const [type, file] of [['PromptDocument', 'document'], ['Policy', 'policy']]) {
  const schema = TJS.generateSchema(program, type, { required: true, noExtraProps: true });
  if (!schema) throw new Error(`Could not generate ${type}`);
  schema.title = `HTMLP ${type} 0.1`;
  schema.$id = `https://alexmckenley.github.io/htmlp/schema/${file}.schema.json`;
  // TypeScript numbers do not express integer constraints; annotate budget and
  // position fields on the generated contract with their normative ranges.
  const walk = obj => {
    if (!obj || typeof obj !== 'object') return;
    for (const [name, prop] of Object.entries(obj.properties ?? {})) {
      if (name.startsWith('max') || ['line', 'column', 'offset'].includes(name)) {
        prop.type = 'integer'; prop.minimum = ['line', 'column'].includes(name) ? 1 : 0;
        prop.maximum = Number.MAX_SAFE_INTEGER;
      }
    }
    for (const value of Object.values(obj)) if (typeof value === 'object') walk(value);
  };
  walk(schema);
  const output = JSON.stringify(schema, null, 2) + '\n';
  const target = `schema/${file}.schema.json`;
  if (process.argv.includes('--check')) {
    if (await readFile(target, 'utf8') !== output) throw new Error(`${target} is stale. Run npm run schema.`);
  } else await writeFile(target, output);
}
