# Floci UI

Rust desktop UI for inspecting a local [Floci](https://github.com/floci-io/floci)
AWS-compatible emulator.

The app is now a Tauri 2 desktop shell with a Leptos CSR frontend built by Trunk.
The native Rust side owns Floci connectivity and exposes commands to the UI.

## Requirements

- Rust 1.95 or newer
- `wasm32-unknown-unknown` Rust target
- [Trunk](https://trunkrs.dev/)
- Node.js and npm for Tailwind CSS
- Tauri 2 CLI and platform prerequisites
- Docker, if you want to run Floci locally through Compose

## Setup

```powershell
Copy-Item .env.example .env
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install tauri-cli --version "^2.0"
npm install
```

## Run With Floci

Start the emulator:

```powershell
docker compose up -d floci
```

Then run the desktop app:

```powershell
cargo tauri dev
```

Tauri starts the npm frontend workflow, which runs Tailwind CSS and Trunk on:

```text
http://localhost:1420
```

By default the native backend checks:

```text
http://localhost:4566/_floci/health
```

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `FLOCI_AWS_ENDPOINT_URL` | `http://localhost:4566` | Local Floci endpoint |
| `FLOCI_AWS_REGION` | `us-east-1` | Region shown by the UI and used by future AWS SDK clients |
| `AWS_ACCESS_KEY_ID` | `test` | Local AWS access key for future inventory clients |
| `AWS_SECRET_ACCESS_KEY` | `test` | Local AWS secret key for future inventory clients |

For safety, `FLOCI_AWS_ENDPOINT_URL` is restricted to loopback hosts plus Floci and
LocalStack localhost aliases such as `localhost.floci.io`,
`*.localhost.floci.io`, and `*.localhost.localstack.cloud`.

## Development Checks

```powershell
npm run tailwind:build
npm run build
cargo +1.95.0 fmt --all --check
cargo +1.95.0 fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo +1.95.0 check --target wasm32-unknown-unknown
cargo +1.95.0 check --manifest-path src-tauri/Cargo.toml
cargo +1.95.0 clippy --target wasm32-unknown-unknown --all-features
cargo +1.95.0 clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features
cargo +1.95.0 test --manifest-path src-tauri/Cargo.toml --all-features
```

## Native Commands

The Tauri backend exposes:

- `floci_health` for the local Floci health endpoint
- `service_catalog` for endpoint, region, credential status, and supported services

The next implementation step is to add AWS SDK-backed inventory commands for S3,
SQS, DynamoDB, Lambda, and IAM.
