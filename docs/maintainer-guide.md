# Maintainer Guide

This guide records the initial maintainer workflow for Floci UI. The initial
owner and maintainer is `godkingjay` until maintainers replace this document.

## Triage

- Confirm whether the report is a bug, feature request, support question, or
  security report.
- Move security reports out of public issues and into GitHub private
  vulnerability reporting or security advisories.
- Ask for operating system, Floci UI version, Floci version, endpoint URL shape,
  reproduction steps, and logs when bug reports are incomplete.
- Close reports about production AWS behavior unless they identify a local
  emulator safety issue in Floci UI.

## Review Rules

- Require at least one maintainer approval before merge.
- Require conversation resolution before merge.
- Keep `master` protected.
- Treat endpoint validation, credential handling, packaging, and GitHub workflow
  changes as higher-risk areas.
- Require screenshots or UI notes for visible UI changes.

## Release Ownership

The release owner is `godkingjay` until a maintainer group replaces that role.
Follow [release-process.md](release-process.md) for versioning, changelog, tags,
artifacts, and rollback.

## Security Escalation

Use GitHub private vulnerability reporting and security advisories instead of
public issues. For confirmed vulnerabilities:

- Create a private advisory.
- Assign a maintainer.
- Reproduce the issue on a local checkout.
- Prepare a fix branch.
- Coordinate disclosure timing in the advisory.
- Publish the advisory after a fixed release is available or when maintainers
  decide disclosure is required.

## Dependency Updates

- Review Rust, npm, and GitHub Actions dependency updates weekly when automation
  is enabled.
- Prefer small grouped updates that are easy to revert.
- Run the relevant checks before merge.
- Treat security updates as priority work.

## Deprecation Policy

Before removing a user-facing command, supported local endpoint alias, or
documented workflow, open an issue and document the replacement. Breaking changes
should wait for a minor release unless the change fixes a security issue.
