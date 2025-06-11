set shell := ["bash", "-cu"]
set dotenv-load := true

# common configuration
container := "taikoscope-masaya"
ssh_alias := "taikoscope"
remote_dir := "~/masaya/taikoscope"
env_file := "$HOME/masaya/taikoscope/masaya.env"
port := "48100:3000"

# display a help message about available commands
default:
    @just --list --unsorted

# start the Taikoscope binary for local development
dev:
    ENV_FILE=dev.env cargo run -- --reset-db

# start the API server for local development
dev-api:
    ENV_FILE=dev.env cargo run --bin api-server

# start the Taikoscope binary with Masaya testnet config
masaya:
    ENV_FILE=masaya.env cargo run

# start the API server with Masaya testnet config
masaya-api:
    ENV_FILE=masaya.env cargo run --bin api-server

# run all recipes required to pass CI workflows
ci:
    @just fmt lint lint-dashboard test check-dashboard test-dashboard

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
        --env-file \{{env_file}} \
        -p {{port}} \
        {{container}}"

set-log-level level:
    @echo "Setting log level to {{level}} on remote server..."
    ssh {{ssh_alias}} "grep -q '^RUST_LOG=' \{{env_file}} && \
        sed -i 's/^RUST_LOG=.*/RUST_LOG={{level}}/' \{{env_file}} || \
        echo 'RUST_LOG={{level}}' >> \{{env_file}}"
    @just start-remote-masaya
    @echo "Log level set to {{level}} and service restarted."

# deploy Taikoscope via SSH alias '{{ssh_alias}}'
deploy-remote-masaya:
    @echo "Deploying Taikoscope via SSH alias '{{ssh_alias}}'"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)
    ssh {{ssh_alias}} "mkdir -p {{remote_dir}}"
    rsync -av --exclude target --exclude .git --exclude dashboard . {{ssh_alias}}:{{remote_dir}}
    @echo "Building Taikoscope on {{ssh_alias}} (path: {{remote_dir}})"
    ssh {{ssh_alias}} "cd {{remote_dir}} && docker buildx build --load -t {{container}} ."
    @just start-remote-masaya

# Check the status of the service
status-remote-masaya:
    ssh {{ssh_alias}} "docker ps -f name={{container}}"

# View the logs of the service
logs-remote-masaya:
    ssh {{ssh_alias}} "docker logs --tail 1000 -f {{container}}"

# Deploy and tail logs
deploy-logs-remote-masaya:
    @just deploy-remote-masaya
    @just logs-remote-masaya

# Start the remote masaya service (runs a new container from the existing image)
start-remote-masaya:
    @echo "Starting Taikoscope masaya service on remote..."
    @just stop-container
    @just run-container
    @echo "Taikoscope masaya service started."

# Stop and remove the remote masaya service
stop-remote-masaya:
    @just stop-container

# Set log level to debug on remote server and restart the service
debug-log-remote-masaya:
    @just set-log-level debug

# Set log level to info on remote server and restart the service
info-log-remote-masaya:
    @just set-log-level info

# Search in logs for a specific term
search-logs-remote-masaya term:
    ssh {{ssh_alias}} "docker logs {{container}} | grep -i \"{{term}}\""

# --- Holesky Deployment (Fly.io) ---

# Deploy Taikoscope to Holesky using Fly.io
deploy-holesky:
    @echo "Deploying Taikoscope to Holesky via Fly.io"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    fly deploy

# Deploy API server to Holesky using Fly.io
deploy-api-holesky:
    @echo "Deploying API server to Holesky via Fly.io"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    fly deploy --config fly-api.toml

# View logs for Holesky deployment
logs-holesky:
    fly logs

# View logs for API server on Holesky
logs-api-holesky:
    fly logs --config fly-api.toml

# Check status of Holesky deployment
status-holesky:
    fly status

# Check status of API server on Holesky
status-api-holesky:
    fly status --config fly-api.toml

# --- API Server Deployment (Masaya) ---
api_container := "taikoscope-api-masaya"
api_port := "48101:3000"

stop-api-container:
    ssh {{ssh_alias}} "docker stop {{api_container}} || true"
    ssh {{ssh_alias}} "docker rm {{api_container}} || true"

run-api-container:
    ssh {{ssh_alias}} "docker run -d \
        --name {{api_container}} \
        --restart unless-stopped \
        --env-file \{{env_file}} \
        -e HOST=0.0.0.0 \
        -p {{api_port}} \
        {{api_container}}"

deploy-api-remote-masaya:
    @echo "Deploying API server via SSH alias '{{ssh_alias}}'"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)
    ssh {{ssh_alias}} "mkdir -p {{remote_dir}}"
    rsync -av --exclude target --exclude .git --exclude dashboard . {{ssh_alias}}:{{remote_dir}}
    @echo "Building API server on {{ssh_alias}} (path: {{remote_dir}})"
    ssh {{ssh_alias}} "cd {{remote_dir}} && docker buildx build --load -f Dockerfile.api -t {{api_container}} ."
    @just start-api-remote-masaya

start-api-remote-masaya:
    @echo "Starting API server on remote..."
    @just stop-api-container
    @just run-api-container
    @echo "API server started."

logs-api-remote-masaya:
    ssh {{ssh_alias}} "docker logs --tail 1000 -f {{api_container}}"

# Deploy and tail logs for the API server
deploy-logs-api-remote-masaya:
    @just deploy-api-remote-masaya
    @just logs-api-remote-masaya

status-api-remote-masaya:
    ssh {{ssh_alias}} "docker ps -f name={{api_container}}"

# --- Dashboard ---

# install dashboard dependencies
install-dashboard:
    cd dashboard && npm install

# start the dashboard dev server
dev-dashboard:
    cd dashboard && VITE_API_BASE="http://localhost:3000" npm run dev

# build the dashboard for production
build-dashboard:
    cd dashboard && npm run build

# run TypeScript type checks
check-dashboard:
    cd dashboard && npm run check

# run dashboard tests
test-dashboard:
    cd dashboard && npm run test

# lint dashboard files for trailing whitespace
lint-dashboard:
    cd dashboard && npm run lint:whitespace
