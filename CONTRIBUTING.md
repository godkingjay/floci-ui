# Contributing to Floci UI

Thank you for helping improve Floci UI. This project is a Rust desktop app for
local Floci emulator workflows, so contributions should keep local endpoint
safety and emulator scope in mind.

## Ground Rules

- Start with an issue unless the change is a small documentation fix.
- Keep pull requests focused and reviewable.
- Do not commit `.env`, local emulator data, `target/`, `dist/`, generated
  bundles, `node_modules/`, generated CSS, logs, screenshots, or local key
  material.
- Do not weaken the loopback-only endpoint checks without maintainer approval.
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Local Setup

PowerShell:

```powershell
Copy-Item .env.example .env
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install tauri-cli --version "^2.0"
npm install
```

POSIX shell:

```sh
cp .env.example .env
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install tauri-cli --version "^2.0"
npm install
```

## Run the App

Start Floci:

```powershell
docker compose up -d floci
```

Run the desktop app:

```powershell
cargo tauri dev
```

The frontend dev server runs on `http://localhost:1420`. The backend defaults to
`http://localhost:4566`.

## Branches and Commits

- Default branch: `master`.
- Branch names should be short and descriptive, for example
  `feature/service-inventory`, `fix/endpoint-validation`, or
  `docs/development-guide`.
- Use clear commit messages that describe the behavior or documentation change.

## Pull Request Expectations

Every pull request should include:

- A concise summary.
- Linked issue when available.
- Screenshots or UI notes for visible changes.
- Validation commands that were run.
- Notes about risk, migration, or follow-up work.

For documentation-only changes, say that Rust and npm builds were not changed.

## Checks

Run the checks that match your change:

```powershell
npm ci
npm run build
cargo +1.95.0 fmt --all --check
cargo +1.95.0 fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo +1.95.0 check --target wasm32-unknown-unknown
cargo +1.95.0 check --manifest-path src-tauri/Cargo.toml
cargo +1.95.0 clippy --target wasm32-unknown-unknown --all-features
cargo +1.95.0 clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features
cargo +1.95.0 test --manifest-path src-tauri/Cargo.toml --all-features
```

If you cannot run a check locally, explain why in the pull request.

## Rust Style

- Use Rust 1.95.0.
- Keep `edition = "2024"`.
- Respect the manifest lints in `Cargo.toml` and `src-tauri/Cargo.toml`.
- Do not use `unwrap()` in library code where an error can be returned.
- Every unsafe block must include a `// SAFETY:` comment.

## Frontend Style

- Keep the Leptos frontend in `src/`.
- Keep native Tauri behavior in `src-tauri/`.
- Use npm for Tailwind CSS and Trunk orchestration only. The package remains
  private because this repository publishes a desktop app, not an npm library.

## Security

Report vulnerabilities through [SECURITY.md](SECURITY.md). Do not open public
issues for suspected vulnerabilities.
