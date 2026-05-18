# Changelog

All notable changes to Floci UI will be documented in this file.

This project follows the spirit of Keep a Changelog and uses semantic version
tags when releases are published.

## [Unreleased]

No notable changes yet.

## [0.1.2] - 2026-05-17

### Added

- Tauri app update manager and update UI surface.
- Focused manual Tauri check workflow for release validation.

### Changed

- Streamlined CI around release checks and credit-heavy validation jobs.
- Refreshed release, packaging, supply-chain, and development documentation.
- Reduced Clippy noise across the Rust codebase and service adapters.

### Fixed

- Corrected runtime refresh and endpoint handling.
- Made development port checks non-destructive, so the app can report port
  conflicts without killing unrelated processes.
- Fixed packaged Tauri app rendering.

### Security

- Resolved dependency security findings.
- Hardened the Tauri content security policy.

## [0.1.0] - 2026-05-16

### Added

- Initial public source release for the Floci UI desktop app.
- Tauri 2 desktop shell with Leptos CSR frontend.
- Local Floci health and service inspection commands.
- Public open source documentation set.
- MIT license file.
- Contribution, security, support, architecture, development, maintainer,
  repository settings, release, supply-chain, and packaging guides.

### Changed

- Expanded README with project scope, setup, checks, troubleshooting, and local
  endpoint safety guidance.

[Unreleased]: https://github.com/godkingjay/floci-ui/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/godkingjay/floci-ui/releases/tag/v0.1.2
[0.1.0]: https://github.com/godkingjay/floci-ui/releases/tag/v0.1.0
