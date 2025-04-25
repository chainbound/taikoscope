set shell := ["bash", "-cu"]
set dotenv-load := true

# display a help message about available commands
default:
    @just --list --unsorted


# run all recipes required to pass CI workflows
ci:
    @just fmt lint test

# run unit tests
test:
    cargo nextest run --workspace -E "kind(lib) | kind(bin) | kind(proc-macro)"

# run collection of clippy lints
lint:
    RUSTFLAGS="-D warnings" cargo clippy --examples --tests --benches --all-features --locked

# format the code using the nightly rustfmt (as we use some nightly lints)
fmt:
    cargo +nightly fmt --all
