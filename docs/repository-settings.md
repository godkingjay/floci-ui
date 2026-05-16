# Repository Settings

Several open source readiness settings must be configured in GitHub rather than
committed as files. Use this document as the expected settings record.

## General

- [ ] Default branch: `master`.
- [ ] Repository description: `Rust desktop UI for inspecting a local Floci AWS-compatible emulator`.
- [ ] Topics: `rust`, `tauri`, `leptos`, `trunk`, `tailwindcss`, `aws`,
  `local-emulator`, `floci`, `desktop-app`.
- [ ] Issues: enabled.
- [ ] Discussions: disabled unless maintainers commit to monitoring them.
- [ ] Wiki: maintainer decision.
- [ ] Sponsorship or funding: not configured for the initial public launch.

## Branch Protection

Protect `master` with:

- [ ] Pull requests required before merge.
- [ ] At least one approval required.
- [ ] Stale approvals dismissed when relevant.
- [ ] Conversation resolution required.
- [ ] Required status check: `CI / rust`.
- [ ] Branch must be up to date before merge when practical.
- [ ] Force pushes blocked.
- [ ] Deletions blocked.

## Security

- [ ] GitHub secret scanning enabled when available.
- [ ] Push protection enabled when available.
- [ ] Private vulnerability reporting enabled when available.
- [ ] Security advisories enabled.
- [ ] Dependabot security updates enabled.

## Labels

Create or verify labels:

- `bug`
- `documentation`
- `dependencies`
- `enhancement`
- `good first issue`
- `help wanted`
- `rust`
- `npm`
- `github-actions`
- `security`
- `tauri`
- `frontend`
- `backend`
- `needs-triage`

## Access

- [ ] `godkingjay` is the initial maintainer.
- [ ] CODEOWNERS or repository rules are updated when maintainer teams are
  created.
- [ ] Release permissions are limited to maintainers.
