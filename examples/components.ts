import { document, system, user, section, text, variable, lint, render } from '../src/index.js';
const prompt = document([
  system([section('role', [text('Review code carefully. Explain concrete issues.')], 200)]),
  user([text('Review this diff:\n'), variable('diff', 2000)]),
], { maxChars: 3000 });
console.log(lint(prompt));
console.log(render(prompt, { diff: '+ const answer = 42;' }));
