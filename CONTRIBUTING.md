# Contributing to Peerino

Thank you for your interest in contributing to Peerino! This guide covers
the development setup, testing, and pull request process.

## Development Setup

Peerino uses **Tauri 2.0** (Rust backend) + **TypeScript/Vite** (frontend).
You need:

- [Node.js](https://nodejs.org/) (LTS, 20+)
- [Rust](https://rust-lang.org/) (latest stable)
- [Cargo](https://doc.rust-lang.org/cargo/) (bundled with Rust)
- Windows 10 or 11 (64-bit) — the app is Windows-only for now

### Install dependencies

```bash
npm install
```

### Local development

```bash
npm run tauri dev
```

This starts the Vite dev server and the Tauri app in development mode.

## Building from source

```bash
# Frontend
npm run build

# Backend (release)
npm run tauri build
```

The output goes to `src-tauri/target/release/`.

## Testing

### Unit tests (Rust)

```bash
cd src-tauri && cargo test
```

### Frontend tests

```bash
npm test
```

## Pull Request Process

1. Fork the repository and create a branch from `main`.
2. Make your changes. Keep commits focused — one logical change per commit.
3. Ensure `cargo check` and `npm run build` pass.
4. Open a Pull Request against `main`. Describe what you changed and why.
5. A maintainer will review your PR. Address any feedback.

## Coding conventions

- **Rust**: follow the existing style. Use `cargo fmt` before committing.
- **TypeScript**: use 4 spaces, no semicolons (matching the existing codebase).
- **Comments**: prefer English for new comments and commit messages.
      Existing Italian comments may be translated opportunistically.
- **License**: by contributing, you agree that your contributions are
  licensed under [AGPL-3.0-or-later](LICENSE).

## Reporting issues

Use the GitHub issue tracker. Include:

- Peerino version (`npm run tauri -- --version` or the About dialog)
- OS and architecture (e.g. Windows 11 x64)
- Steps to reproduce
- Expected vs. actual behavior
- Screenshots or logs if available

## Questions?

Open a Discussion on GitHub (not an issue) for questions that are not
bug reports.

## Contributor License Agreement

Before your contribution can be accepted, you must agree to the terms below.
This is a lightweight CLA based on the Apache Individual CLA and the
[Harmony Agreements](https://www.harmonyagreements.org/) (License option).

**By submitting a pull request, patch, or any other contribution to this
repository, you agree that:**

1. **Authorship.** You are the author of the contribution, or you have the
   legal right to submit it under these terms.

2. **Copyright license.** You grant Daniele Tomassoni and the Peerino
   project a perpetual, worldwide, non-exclusive, royalty-free, irrevocable
   copyright license to reproduce, modify, and distribute your contribution,
   to prepare derivative works of it, and to **sublicense and relicense**
   it under any license, including proprietary licenses.

3. **Retained ownership.** You retain copyright of your contribution. This
   is a license grant, not a copyright assignment. You may reuse your
   contribution elsewhere under any terms you like.

4. **Patent license.** You grant a patent license for any patents you own
   that are necessarily infringed by your contribution, on the same terms
   as the copyright license above.

5. **No warranty.** Contributions are provided "as is", without warranty of
   any kind.

6. **Governing law.** This CLA is governed by the laws of Italy, without
   regard to conflict-of-law principles.

**Why this CLA?** It lets Peerino stay free and open source (AGPL-3.0) for
the community, while preserving the option to offer a commercial license
in the future. Without it, every contributor would retain veto power over
any future licensing decision.

If you cannot agree to these terms, please open an issue instead of a pull
request describing the change you would like to see.