# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| `0.1.x` | Yes |
| `<0.1.0` | No |

## Reporting a Vulnerability

Please report security issues privately through GitHub Security Advisories:

1. Open the repository `Security` tab.
2. Create a private vulnerability report with reproduction steps and impact.
3. Include affected version(s), platform, and proof-of-concept details.

If Security Advisories are unavailable, open a private maintainer contact issue and avoid publishing exploit details.

## Response Process

1. We acknowledge valid reports within 3 business days.
2. We triage severity and impacted versions.
3. We ship a fix and publish an advisory once users can patch safely.

## Known Upstream Dependency Risk

Current Linux desktop dependencies pulled by `wry`/`tao` include GTK3-related crates with RustSec warnings (unmaintained/unsound notices). These are tracked in `scripts/ci/cargo-audit-ignore.json` and are temporarily allowlisted until upstream migration removes the affected stack.
