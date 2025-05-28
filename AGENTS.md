# Agent Guidelines for Taikoscope

## Build & Test Commands
- Build & run: `just dev` (with dev.env file) or `cargo run`
- API server: `just dev-api` or `cargo run --bin api-server`
- Run tests: `just test` or `cargo nextest run --workspace --all-targets`
- Run single test: `cargo nextest run <test_name>` or `cargo test <test_name>`
- Linting: `just lint` or `cargo clippy --examples --tests --benches --all-features`
- Format code: `just fmt` or `cargo +nightly fmt --all`
- Run CI checks: `just ci` (runs fmt, lint, test)
- Always run all CI checks after any changes (except for changes in the dashboard dir)
- Dashboard install dependencies: `just install-dashboard`
- Dashboard dev server: `just dev-dashboard`
- Dashboard build: `just build-dashboard`
- Dashboard type checks: `just check-dashboard`

## Code Style Guidelines
- Use Rust 2024 edition
- Follow rustfmt.toml settings: reordered imports, grouped by crate, use small heuristics
- Missing debug impls and docs should be warned
- Follow Clippy lints defined in Cargo.toml
- Errors use `eyre` crate
- Tests: Use `#[cfg(test)]` module and functions with `#[test]` or `#[tokio::test]`
- Async: Use tokio for async runtime
- Prefer `#[derive(Debug)]` on structs and enums
- Use trace/debug/info/warn/error logs properly with `tracing` crate
- Error handling: Prefer `?` operator with contextual error info

## Git
- Use Conventional Commits for commits
