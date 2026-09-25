"""Use HTMLP from Python through its portable JSON interface (run from repo root)."""
import json
import subprocess
messages = json.loads(subprocess.check_output([
    "node", "dist/cli.js", "render", "examples/rules.htmlp",
    "--vars", "examples/variables.json"
], text=True))
print(json.dumps(messages, indent=2))
