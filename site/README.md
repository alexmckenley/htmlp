# HTMLP website

Plain HTML and CSS. Build from the repository with `python3 site/build.py`; it generates the Rust reference and copies the schema/specification. Serve `site/dist` with any static server. For GitHub Pages, set `HTMLP_BASE_PATH=/htmlp`.

The separate Sites source checkout includes `reference/` as a generated snapshot so it can build independently. Its `.openai/hosting.json` selects `dist` as the static output.
