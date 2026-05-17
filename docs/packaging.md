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

The GitHub release workflow builds each desktop target on its native hosted
runner. Linux bundles are built on Ubuntu, Windows bundles on Windows, and macOS
bundles on macOS.

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

The app bundle version is sourced from `package.json` through
`src-tauri/tauri.conf.json`. Bump `package.json`, `Cargo.toml`, and
`src-tauri/Cargo.toml` before tagging a release so artifact names, updater
metadata, and the app-reported current version stay aligned.

The automated release workflow uploads these artifacts to the draft GitHub
release through the Tauri release action:

- Linux: `.AppImage`, `.deb`, and `.rpm`.
- Windows: `.msi` and `.exe`.
- macOS: `.dmg` and `.app.tar.gz`.
- Updater metadata: `latest.json`.
- Updater signatures: `.sig` files for signed updater bundles.

`latest.json` is the metadata file installed apps read to discover available
updates. Do not publish the draft release until this file and the expected
signature files are present.

## Signing and Notarization

Signing, notarization, and store distribution are not configured for the initial
public source release. Unsigned local builds are suitable for maintainer testing
and contributor validation only.

Updater signing is separate from platform code signing. Release builds use
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from GitHub
Actions secrets so Tauri can create updater signatures and `latest.json`.

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
- Verify updater metadata and `.sig` files are present on the draft release.
- Record artifact checksums.
