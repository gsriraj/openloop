# Contributing to OpenLoop

Thank you for considering contributing! Here's how to get started.

## Development Setup

```bash
# Clone and build
git clone https://github.com/gsriraj/openloop.git
cd openloop
cargo build

# Run tests
cargo test

# Check formatting and linting
cargo fmt --check
cargo clippy -- -D warnings
```

## Commit Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add interactive setup wizard
fix: handle missing agent CLI gracefully
docs: update README with examples
test: add integration test for engine loop
chore: bump clap to 4.5
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `ci`, `style`

## Pull Request Process

1. Fork the repo and create a feature branch
2. Make your changes
3. Run `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings`
4. Submit a PR with a clear description

## Code Style

- Follow existing patterns in the codebase
- Use `anyhow::Result` for fallible functions
- Add `#[cfg(test)] mod tests` in every module
- Prefer clarity over brevity

## Testing

- Every module should have unit tests
- Integration tests live in `tests/`
- Use `tests/fixtures/mock-agent.sh` for end-to-end tests

## Questions?

Open an issue with the `question` label.