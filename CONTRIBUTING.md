# Contributing to HTMLP

Start with a bug report, a small documentation fix, or a concrete example of prompt growth that the current contract cannot express. Discuss syntax and counting changes in an issue before implementing them.

Use Node.js 22.13+ and npm. Run `npm ci`, then `npm run check`. For the site, also run `npm --prefix site ci` and `npm run docs:build`. Run examples from the repository root. After changing public types, run `npm run schema` and commit schema changes. API documentation is generated during the site build and is not hand-edited.

Include a focused test for behavior changes. Parser changes should include accepted and rejected examples. Budget changes should cover boundary cases, Unicode, variables, and inherited policy. Keep tests deterministic and independent of live models or external services.

Describe the problem, resulting behavior, and validation in each pull request. Avoid unrelated formatting and dependency changes. A maintainer reviews contributions before merging. Contributions use the project's MIT license; no CLA is required.

The 0.1 format is experimental. Breaking changes must update the specification, examples, schema, and changelog together. Future stable releases will use semantic versioning.
