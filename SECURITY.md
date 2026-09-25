# Security policy

HTMLP is experimental; only the latest commit and latest prerelease receive fixes. It does not execute templates, fetch remote resources, or call models. Treat repository files and filesystem access as trusted input. Runtime string bindings do not become markup, but may still contain adversarial natural-language instructions.

Do not report vulnerabilities with private data in public issues. Use GitHub private vulnerability reporting at https://github.com/alexmckenley/htmlp/security/advisories/new. Include the affected version, a minimal reproduction, expected behavior, and impact. Maintainers will investigate and coordinate a fix where possible; no response-time guarantee or bounty is offered.

Budget checks do not replace provider token counting, provenance controls, filesystem isolation, or protection of policy files in code review.
