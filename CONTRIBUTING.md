# Contributing to Peerino

Thank you for your interest in contributing to Peerino! This guide covers
the development setup, testing, and pull request process.

## Development Setup

Peerino uses **Tauri 2.0** (Rust backend) + **TypeScript/Vite** (frontend).
You need:

- [Node.js](https://nodejs.org/) (LTS, 18+)
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
- **Comments**: write in English. Avoid Italian in code or commit messages.
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