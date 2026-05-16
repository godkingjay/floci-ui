# Architecture

Floci UI is a local desktop app for inspecting a local Floci AWS-compatible
emulator. It is intentionally scoped to local emulator workflows and should not
be treated as a production AWS console.

## Runtime Shape

```text
Leptos UI (src/)
  |
  | Tauri invoke
  v
Tauri backend (src-tauri/)
  |
  | reqwest and AWS SDK clients
  v
Local Floci endpoint, usually http://localhost:4566
```

## Frontend

The frontend lives in `src/` and is compiled to WASM by Trunk. It uses Leptos CSR
for the desktop UI and npm scripts for Tailwind CSS and Trunk orchestration.

Important directories:

- `src/views/`: top-level app screens.
- `src/components/`: reusable UI components.
- `src/service_management/`: frontend models, client code, domains, and service
  management views.

The frontend does not connect directly to Floci. It calls Tauri commands exposed
by the native backend.

## Tauri Backend

The backend lives in `src-tauri/`. It owns:

- Environment loading through `.env`.
- Endpoint URL validation.
- Local Floci health checks.
- AWS SDK client setup for local emulator service adapters.
- Tauri command registration.

Commands currently include:

- `floci_health`
- `service_catalog`
- `service_inventory`
- `service_resource_detail`
- `service_execute_action`

The backend returns serialized models to the frontend and avoids exposing the
secret access key in dashboard snapshots.

## Service Adapter Model

Service management code is organized under `src-tauri/src/service_management/`.
Adapters are grouped by domain, such as data storage, compute and build,
messaging and events, network and observability, and security configuration.

Adapters should describe local emulator state and actions. They should not assume
full production AWS compatibility because Floci UI follows the capabilities of
the local emulator.

## Local Endpoint Safety

`FLOCI_AWS_ENDPOINT_URL` defaults to `http://localhost:4566`. The backend accepts
loopback IPs and local emulator aliases:

- `localhost`
- `127.0.0.1`
- `[::1]`
- `floci`
- `host.docker.internal`
- `localhost.floci.io`
- `*.localhost.floci.io`
- `localhost.localstack.cloud`
- `*.localhost.localstack.cloud`

Remote hosts are rejected before the app starts. This protects contributors from
accidentally pointing the UI at production or shared AWS-compatible endpoints.

## Packaging Boundary

The root Rust package builds the frontend WASM app. `src-tauri/Cargo.toml` builds
the native desktop shell and backend. `package.json` remains private because npm
is only used for build tooling.
