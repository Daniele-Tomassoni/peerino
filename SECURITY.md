# Security Policy

## Supported versions

Only the latest release of Peerino receives security updates.

| Version | Supported |
|---------|-----------|
| 1.0.x   | ✅        |
| < 1.0   | ❌        |

## Reporting a vulnerability

Please **do not** report security issues as public GitHub issues.

Use GitHub's [private vulnerability reporting](https://github.com/Daniele-Tomassoni/peerino/security/advisories/new)
form. This opens a private channel between you and the maintainer, where
the report stays confidential until a fix is published.

Include in your report:

- A description of the issue
- Steps to reproduce
- The version of Peerino affected
- Any proof-of-concept code or screenshots

You will receive a response within 7 days. If the issue is confirmed, a
fix will be released as soon as possible, and you will be credited in the
release notes (unless you prefer to remain anonymous).

## Scope

In scope:

- Peerino desktop application (Tauri/Rust + TypeScript frontend)
- Web receiver (`peerino.com/receiver.html`)
- TURN credential proxy Worker
- Data integrity of file transfers

Out of scope:

- Denial of service against Cloudflare's infrastructure
- Issues in third-party dependencies (report upstream)
- Social engineering