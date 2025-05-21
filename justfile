set shell := ["bash", "-cu"]
set dotenv-load := true

# common configuration
container := "taikoscope-hekla"
ssh_alias := "taikoscope"
remote_dir := "~/hekla/taikoscope"
env_file := remote_dir + "/masaya.env"
port := "48100:3000"

# display a help message about available commands
default:
    @just --list --unsorted

# start the Taikoscope binary for local development
dev:
    ENV_FILE=dev.env cargo run -- --reset-db

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

# internal helpers
stop-container:
    ssh {{ssh_alias}} "docker stop {{container}} || true"
    ssh {{ssh_alias}} "docker rm {{container}} || true"

run-container:
    ssh {{ssh_alias}} "docker run -d \
        --name {{container}} \
        --restart unless-stopped \
        --env-file {{env_file}} \
        -p {{port}} \
        {{container}}"

set-log-level level:
    @echo "Setting log level to {{level}} on remote server..."
    ssh {{ssh_alias}} "grep -q '^RUST_LOG=' {{env_file}} && \
        sed -i 's/^RUST_LOG=.*/RUST_LOG={{level}}/' {{env_file}} || \
        echo 'RUST_LOG={{level}}' >> {{env_file}}"
    @just start-remote-hekla
    @echo "Log level set to {{level}} and service restarted."

# deploy Taikoscope via SSH alias '{{ssh_alias}}'
deploy-remote-hekla:
    @echo "Deploying Taikoscope via SSH alias '{{ssh_alias}}'"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)
    ssh {{ssh_alias}} "mkdir -p {{remote_dir}}"
    rsync -av --exclude target --exclude .git . {{ssh_alias}}:{{remote_dir}}
    @echo "Building Taikoscope on {{ssh_alias}} (path: {{remote_dir}})"
    ssh {{ssh_alias}} "cd {{remote_dir}} && docker buildx build --load -t {{container}} ."
    @just start-remote-hekla

# Check the status of the service
status-remote-hekla:
    ssh {{ssh_alias}} "docker ps -f name={{container}}"

# View the logs of the service
logs-remote-hekla:
    ssh {{ssh_alias}} "docker logs -f {{container}}"

# Deploy and tail logs
deploy-logs-remote-hekla:
    @just deploy-remote-hekla
    @just logs-remote-hekla

# Start the remote Hekla service (runs a new container from the existing image)
start-remote-hekla:
    @echo "Starting Taikoscope Hekla service on remote..."
    @just stop-container
    @just run-container
    @echo "Taikoscope Hekla service started."

# Stop and remove the remote Hekla service
stop-remote-hekla:
    @just stop-container

# Set log level to debug on remote server and restart the service
debug-log-remote-hekla:
    @just set-log-level debug

# Set log level to info on remote server and restart the service
info-log-remote-hekla:
    @just set-log-level info

# Search in logs for a specific term
search-logs-remote-hekla term:
    ssh {{ssh_alias}} "docker logs {{container}} | grep -i \"{{term}}\""
