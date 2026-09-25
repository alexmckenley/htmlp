# HTMLP website

Plain HTML and CSS. Build from the repository with `python3 site/build.py`; it generates the Rust reference and copies the schema/specification. Serve `site/dist` with any static server. The public site at `htmlp.dev` uses the root path (empty `HTMLP_BASE_PATH`).

`pages/index.html` is the short homepage; `pages/docs/index.html` covers prompt files and features; `pages/docs/rust/index.html` covers the Rust interface. Keep their examples aligned with `docs/rust.md` and the public types. `pages/llms.txt` is the compact agent reference. The build publishes Markdown references, schemas, and Rustdoc alongside the HTML guides.

GitHub Pages deploys this directory through `.github/workflows/pages.yml` in the standalone HTMLP repository. When working in the `sobrcode` subtree, publish HTMLP upstream to trigger that deployment.
