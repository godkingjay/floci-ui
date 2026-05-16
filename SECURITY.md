# Security Policy

## Supported Versions

Floci UI is preparing for its first public release. Until `0.1.0` is released,
only the `master` branch receives security fixes.

| Version | Supported |
| --- | --- |
| `master` | Yes |
| `< 0.1.0` tags | No |

## Reporting a Vulnerability

Do not report suspected vulnerabilities in public issues, discussions, or pull
requests.

Use GitHub private vulnerability reporting when it is available for this
repository. If private reporting is not visible, open a GitHub security advisory
request through the repository security tab or contact a maintainer through a
private channel already available to you.

Include:

- Affected version, branch, or commit.
- Operating system.
- Steps to reproduce.
- Impact and whether local secrets, local emulator data, or host access could be
  affected.
- Any logs, screenshots, or proof-of-concept code that can be shared privately.

## Response Expectations

- Maintainers aim to acknowledge complete reports within 7 calendar days.
- Maintainers aim to provide an initial triage result within 14 calendar days.
- Fix timing depends on severity, reproducibility, and maintainer availability.

These timelines are targets, not service-level guarantees.

## Scope

In scope:

- Floci UI desktop app code in this repository.
- Endpoint validation and local emulator connection handling.
- Handling of local credentials used for emulator workflows.
- Build, packaging, and release scripts maintained in this repository.

Out of scope:

- Production AWS service behavior.
- Vulnerabilities in Floci itself unless Floci UI directly exposes or worsens
  the issue.
- Vulnerabilities requiring a malicious local machine administrator.
- Dependency vulnerabilities that do not affect reachable Floci UI behavior.

## Endpoint Safety Model

`FLOCI_AWS_ENDPOINT_URL` is intended for local emulator endpoints only. The app
allows loopback hosts and local emulator aliases, then rejects remote hosts. Do
not bypass this safety model without a reviewed design change.
