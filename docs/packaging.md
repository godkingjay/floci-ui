# Packaging

Floci UI packages are built through Tauri. Packaging is intended for desktop app
distribution, not npm publication.

## Prerequisites

- Rust 1.95.0.
- Tauri 2 CLI.
- Node.js and npm.
- Trunk.
- Platform-specific Tauri prerequisites.

Linux builds require the system packages documented by Tauri for WebKit, GTK,
and app indicator support. Windows and macOS builds require their normal native
toolchains.

## Local Package Build

Build frontend assets:

```powershell
npm run build
```

Build Tauri bundles:

```powershell
cargo tauri build
```

Tauri writes native build output under `src-tauri/target/`. Generated output is
ignored by git.

## Expected Artifacts

Artifact names and extensions depend on the host platform and installed Tauri
bundlers. Typical outputs include installers or app bundles under
`src-tauri/target/release/bundle/`.

## Signing and Notarization

Signing, notarization, and store distribution are not configured for the initial
public source release. Unsigned local builds are suitable for maintainer testing
and contributor validation only.

## Unsupported Release Targets

The initial release process does not promise:

- npm package publication.
- Production AWS compatibility.
- Mobile builds.
- Browser-only hosted deployment.
- Signed installers unless maintainers add signing credentials.

## Verification

Before attaching an artifact to a release:

- Start a local Floci endpoint.
- Launch the packaged app.
- Verify the app shows the expected endpoint and health status.
- Verify remote endpoints are still rejected.
- Record artifact checksums.
