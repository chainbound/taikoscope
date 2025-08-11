# Agent Guidelines for Taikoscope

## Build & Test Commands
- Build & run: `just dev` (with dev.env file) or `cargo run`
- API server: `just dev-api` or `cargo run --bin api-server`
- Run tests: `just test` or `cargo nextest run --workspace --all-targets`
- Run single test: `cargo nextest run <test_name>` or `cargo test <test_name>`
- Linting: `just lint` or `cargo clippy --examples --tests --benches --all-features`
- Format code: `just fmt` or `cargo +nightly fmt --all`
- Run CI checks: `just ci` (runs fmt, lint, test, check-dashboard, test-dashboard)
- Smart CI: `just ci-smart` (automatically detects changed files and runs only relevant tooling)
- Rust-only CI: `just ci-rust` (runs fmt, lint, test)
- Dashboard-only CI: `just ci-dashboard` (runs lint-dashboard, check-dashboard, test-dashboard)
- Always run `just ci-smart` after changes, or `just ci` for full manual control
- Dashboard install dependencies: `just install-dashboard`
- Dashboard dev server: `just dev-dashboard`
- Dashboard build: `just build-dashboard`
- Dashboard type checks: `just check-dashboard`
- Dashboard tests: `just test-dashboard`

## Docker Build Commands
- Setup multi-platform builds: `just setup-docker` (automatically run by build commands)
- Build processor: `just build-processor` (defaults to multi-platform build)
- Build API: `just build-api` (defaults to multi-platform build)
- Build all images: `just build-all`
- Single platform builds: `just build-processor platform=linux/amd64`

## Code Style Guidelines
- Use Rust 2024 edition
- Sort imports so that internal crates come first. If there are other
  dependencies after the internal ones, add a blank line between the groups.
- Follow rustfmt.toml settings: reordered imports, grouped by crate, use small heuristics
- Missing debug impls and docs should be warned
- Follow Clippy lints defined in Cargo.toml
- Errors use `eyre` crate
- Tests: Use `#[cfg(test)]` module and functions with `#[test]` or `#[tokio::test]`
- Async: Use tokio for async runtime
- Prefer `#[derive(Debug)]` on structs and enums
- Use trace/debug/info/warn/error logs properly with `tracing` crate
- Error handling: Prefer `?` operator with contextual error info
- Avoid lines with trailing whitespace (spaces or tabs)

## Dashboard Style Guidelines
- All dashboard changes must look great in both light and dark modes
- Test UI components and styling in both themes before finalizing changes

## NATS Exactly-Once Configuration
- Publishing uses `publish_event_with_retry()` with 10 retries and exponential backoff (first retry after 1s)
- Each event includes a unique `Msg-Id` header based on `TaikoEvent::dedup_id()`
- For production: configure NATS stream with `duplicate_window: Duration::from_secs(120)` and file storage
- NATS JetStream provides exactly-once delivery using message ID deduplication

## Git
- Use Conventional Commits for commits
