## Summary

- 

## Screenshots or UI Notes

- 

## Validation

- [ ] `npm ci`
- [ ] `npm run build`
- [ ] `cargo +1.95.0 fmt --all --check`
- [ ] `cargo +1.95.0 fmt --manifest-path src-tauri/Cargo.toml --all --check`
- [ ] `cargo +1.95.0 check --target wasm32-unknown-unknown`
- [ ] `cargo +1.95.0 check --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo +1.95.0 clippy --target wasm32-unknown-unknown --all-features`
- [ ] `cargo +1.95.0 clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features`
- [ ] `cargo +1.95.0 test --manifest-path src-tauri/Cargo.toml --all-features`

## Risk

- 

## Linked Issues

- 

## Checklist

- [ ] Rust code follows the project lints and avoids unchecked `unwrap()` in library paths.
- [ ] npm and Trunk build paths were checked when frontend assets changed.
- [ ] Documentation was updated for user-visible behavior.
- [ ] Security-sensitive behavior, endpoints, credentials, and generated artifacts were reviewed.
