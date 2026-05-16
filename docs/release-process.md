# Release Process

Floci UI releases are source and desktop app releases. The npm package remains
private because npm is used for build tooling.

## Versioning

Use semantic version tags such as `v0.1.0`.

Update version values together:

- `Cargo.toml`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `package.json`

Do not change `package.json` from `"private": true` for normal desktop releases.

## Pre-Release Checklist

1. Confirm `master` is green.
2. Update [CHANGELOG.md](../CHANGELOG.md).
3. Verify [SECURITY.md](../SECURITY.md) supported versions.
4. Run the documented development checks.
5. Build packages for each supported platform.
6. Verify artifact names, checksums, and installer behavior.
7. Draft release notes from the changelog and merged pull requests.

## Tagging

Create an annotated tag:

```powershell
git tag -a v0.1.0 -m "Floci UI v0.1.0"
git push origin v0.1.0
```

## Release Notes

Release notes should include:

- Summary.
- User-facing changes.
- Security fixes.
- Known limitations.
- Checksums or artifact verification notes when artifacts are attached.

## Artifact Verification

For each artifact:

- Confirm it starts on the intended platform.
- Confirm it can connect to a local Floci endpoint.
- Confirm it does not require production AWS credentials.
- Record checksums.

Signing and notarization are not required for the first public source release
unless maintainers add platform signing credentials.

## Rollback

If a release is broken:

1. Mark the GitHub release as pre-release or add a warning.
2. Remove broken artifacts if they are unsafe to use.
3. Open a fix issue.
4. Publish a patch tag such as `v0.1.1`.
5. Document the rollback or replacement in the changelog.
