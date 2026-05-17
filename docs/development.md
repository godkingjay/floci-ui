# Development Guide

This guide covers local development for Floci UI.

## Toolchain

- Rust 1.95.0 from `rust-toolchain.toml`.
- `wasm32-unknown-unknown` target for the Leptos frontend.
- Trunk for WASM bundling.
- Node.js and npm for Tailwind CSS.
- Tauri 2 CLI and platform prerequisites.
- Docker for running Floci through Compose.

Install the Rust target:

```powershell
rustup target add wasm32-unknown-unknown
```

Install Rust tools:

```powershell
cargo install trunk
cargo install tauri-cli --version "^2.0"
```

Install npm dependencies:

```powershell
npm ci
```

## Environment

Create a local environment file:

```powershell
Copy-Item .env.example .env
```

The default endpoint is `http://localhost:4566`. Use local emulator endpoints
only.

## Run Floci

```powershell
docker compose up -d floci
```

The Compose service maps Floci to port `4566` and stores local data under
`./data`, which is ignored by git.

## Run the Desktop App

```powershell
cargo tauri dev
```

Tauri runs `npm run dev`, which starts Tailwind watch mode and Trunk on
`http://localhost:1420`.

## Build

```powershell
npm run build
```

## Rust Checks

```powershell
cargo +1.95.0 fmt --all --check
cargo +1.95.0 fmt --manifest-path src-tauri/Cargo.toml --all --check
```

Run backend tests, Clippy, and Tauri checks locally before larger backend or
native-shell refactors. For native-shell validation, you can also run the manual
`Tauri Check` GitHub Actions workflow. The default CI gate keeps formatting and
frontend build only so pull requests do not spend credits on duplicate compile
lanes.

## Common Failures

### Trunk cannot find the WASM target

Run:

```powershell
rustup target add wasm32-unknown-unknown
```

### Tauri fails on Linux prerequisites

Install the Tauri 2 Linux prerequisites for your distribution, including WebKit
and GTK packages. See the Tauri prerequisites documentation.

### Port 1420 is busy

The npm dev script runs `scripts/free-dev-port.mjs` before starting Trunk. If the
port is still unavailable, stop the process using `1420` and rerun
`cargo tauri dev`.

### Endpoint is rejected

Use a loopback URL or supported local emulator alias. Remote endpoints are
blocked by design.

### Disk usage grows during native checks

Remove generated build output:

```powershell
Remove-Item -Recurse -Force target, src-tauri/target, dist -ErrorAction SilentlyContinue
```

Do not remove files you have not generated yourself.
