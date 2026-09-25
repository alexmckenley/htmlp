#!/usr/bin/env python3
"""Build readable static HTML. Python standard library; no frontend runtime."""
from pathlib import Path
import os
import shutil
import subprocess

site = Path(__file__).resolve().parent
root = site.parent
reference = site / 'reference'
if (root / 'Cargo.toml').exists():
    subprocess.run(['cargo', 'doc', '--locked', '--all-features', '--no-deps'], cwd=root, check=True)
    reference.mkdir(exist_ok=True)
    shutil.copytree(root / 'target/doc', reference / 'api', dirs_exist_ok=True)
    for schema in (root / 'schema').glob('*.schema.json'):
        shutil.copyfile(schema, reference / schema.name)
    for name in ['spec.md', 'runtime.md']:
        text = (root / 'docs' / name).read_text().replace('../schema/', './')
        (reference / name).write_text(text)
if not (reference / 'api/htmlp/index.html').is_file():
    raise SystemExit('Build from the HTMLP repository or supply the generated reference directory.')
out = site / 'dist'
if out.exists():
    shutil.rmtree(out)
out.mkdir()
base = os.environ.get('HTMLP_BASE_PATH', '').rstrip('/')
if base and (not base.startswith('/') or any(c not in '/abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_' for c in base)):
    raise SystemExit('HTMLP_BASE_PATH must be a simple absolute URL path.')
for source in (site / 'pages').rglob('*'):
    if source.is_file():
        destination = out / source.relative_to(site / 'pages')
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(source.read_text().replace('{{BASE}}', base))
shutil.copytree(reference, out, dirs_exist_ok=True)
(out / '.nojekyll').touch()
print(f'Built {out}; homepage: {(out / "index.html").stat().st_size} bytes')
