# Security

HTMLP is experimental. Report vulnerabilities privately through [GitHub security advisories](https://github.com/alexmckenley/htmlp/security/advisories/new). Include a minimal reproducer and affected version; do not disclose credentials or private prompt contents.

The parser rejects unsupported markup and does not resolve external entities, execute expressions, or fetch resources. Parsing is bounded to 4 MiB and 64 element levels. Directory scans skip symlinks and limit traversal depth. These bounds reduce common failure modes; they are not a sandbox or a formal security audit.

Runtime variable strings are inserted literally. HTMLP validates their token lengths but does not establish trust or prevent prompt injection. Inline limits and reasons are editable source and must be protected by repository review if changes matter. A tokenizer must match the target use case; counts omit SDK message framing and other provider overhead.

Only the current alpha is actively maintained. Do not use abort-on-panic release binaries as a substitute for process isolation when handling untrusted inputs.
