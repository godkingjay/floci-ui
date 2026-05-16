# Supply Chain

This project depends on Rust crates, npm packages, GitHub Actions, Tauri tooling,
and Docker images used for local development.

## Rust Dependencies

- Review dependency updates before merge.
- Prefer maintained crates with clear licenses.
- Treat AWS SDK, Tauri, networking, archive, and filesystem dependencies as
  higher risk.
- Run `cargo +1.95.0 check`, `cargo +1.95.0 clippy`, and relevant tests after
  updates.
- If `cargo-deny` is adopted, document the policy in `deny.toml` and keep it
  aligned with the current dependency graph.

## npm Dependencies

`package.json` remains private. npm dependencies are build tooling dependencies,
not runtime library dependencies published to npm.

- Use `package-lock.json` for deterministic installs.
- Review Tailwind, Trunk orchestration, and development tool updates.
- Run `npm install` or `npm ci` according to the workflow being validated.
- Run `npm run build` after updates.

## GitHub Actions

- Keep workflow permissions minimal.
- Pin major versions at minimum.
- Review workflow updates like code changes.
- Avoid actions that require broad tokens unless the workflow needs them.

## Advisories

When an advisory affects the project:

1. Confirm whether the vulnerable code path is reachable in Floci UI.
2. Prioritize updates for reachable or build-chain vulnerabilities.
3. Open a focused pull request.
4. Document validation and release impact.

## Current Advisory Exceptions

`deny.toml` records temporary advisory exceptions for transitive dependencies
that are currently inherited through Leptos 0.6, Tauri's Linux GTK stack,
Tauri URL pattern parsing, the test-only `httpmock` graph, or the AWS SDK's
Rustls 0.21 verifier path. The app is still restricted to local HTTP emulator
endpoints, and maintainers should remove each exception as upstream packages
ship compatible fixes.

## Generated Artifacts

Do not commit generated artifacts unless maintainers explicitly decide they are
source-controlled project assets.

Ignored generated paths include:

- `target/`
- `src-tauri/target/`
- `src-tauri/gen/`
- `dist/`
- `coverage/`
- `screenshots/`
- `test-results/`
- `data/`
- `node_modules/`
- `npm-cache/`
- `styles.css`
- `.env`
- `.env.*` except `.env.example`
- logs and local key material

## Local Secrets

Do not commit credentials, private keys, real AWS account data, or local emulator
state. The default `test` credentials are placeholders for local emulator use.
