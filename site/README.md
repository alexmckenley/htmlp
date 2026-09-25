# HTMLP website

Homepage and language guide, built with the Sites Vinext starter and exported as static HTML. The parent repository generates TypeDoc reference pages, schemas, and the standalone specification into `public/` before building.

From the repository root:

```sh
npm ci
npm --prefix site ci
npm run docs:build
npm --prefix site run dev
```

`HTMLP_BASE_PATH=/htmlp npm run docs:build` builds for GitHub Pages. Omitting that variable builds for a root-domain host. GitHub Actions publishes the public site from `main`. Sites deployment uses `.openai/hosting.json` and the static output at `dist/client`.
