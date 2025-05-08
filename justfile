set shell := ["bash", "-cu"]
set dotenv-load := true

# display a help message about available commands
default:
    @just --list --unsorted

# start the Taikoscope binary for local development
dev:
    ENV_FILE=dev.env RUST_LOG=debug cargo run -- --reset-db

# start the Taikoscope binary with Masaya testnet config
masaya:
    ENV_FILE=masaya.env cargo run

# run all recipes required to pass CI workflows
ci:
    @just fmt lint test

# run tests
test:
    cargo nextest run --workspace --all-targets

# run collection of clippy lints
lint:
    RUSTFLAGS="-D warnings" cargo clippy --examples --tests --benches --all-features --locked

# format the code using the nightly rustfmt (as we use some nightly lints)
fmt:
    cargo +nightly fmt --all
