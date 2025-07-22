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
    ENV_FILE=dev.env cargo run --bin processor

# start the API server for local development
dev-api:
    ENV_FILE=hekla.env cargo run --bin api-server

# start local NATS for development
dev-nats:
    #!/usr/bin/env bash
    if docker ps -q -f name=local-nats | grep -q .; then
        echo "NATS container is already running"
    elif docker ps -a -q -f name=local-nats | grep -q .; then
        echo "Starting existing NATS container"
        docker start local-nats
    else
        echo "Creating new NATS container"
        docker run -d --name local-nats -p 4222:4222 -p 8222:8222 nats:latest -js -m 8222
    fi

# stop local NATS
stop-dev-nats:
    docker stop local-nats || true
    docker rm local-nats || true

# start the ingestor for local development
dev-ingestor:
    ENV_FILE=hekla.env cargo run --bin ingestor

# start the processor for local development
dev-processor:
    ENV_FILE=hekla.env SKIP_MIGRATIONS=true ENABLE_DB_WRITES=false cargo run --bin processor

# run complete local NATS pipeline (starts NATS, ingestor, and processor)
dev-pipeline:
    @echo "Starting complete local NATS pipeline..."
    @just dev-nats
    @echo "Waiting for NATS to be ready..."
    @sleep 3
    @echo "NATS ready. Start ingestor and processor manually with:"
    @echo "  just dev-ingestor    # (in terminal 1)"
    @echo "  just dev-processor   # (in terminal 2)"

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
    @just start-masaya
    @echo "Log level set to {{level}} and service restarted."

# deploy Taikoscope via SSH alias '{{ssh_alias}}'
deploy-masaya:
    @echo "Deploying Taikoscope via SSH alias '{{ssh_alias}}'"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)
    ssh {{ssh_alias}} "mkdir -p {{remote_dir}}"
    rsync -av --exclude target --exclude .git --exclude dashboard . {{ssh_alias}}:{{remote_dir}}
    @echo "Building Taikoscope on {{ssh_alias}} (path: {{remote_dir}})"
    ssh {{ssh_alias}} "cd {{remote_dir}} && docker buildx build --load -t {{container}} ."
    @just start-masaya

# Check the status of the service
status-masaya:
    ssh {{ssh_alias}} "docker ps -f name={{container}}"

# View the logs of the service
logs-masaya:
    ssh {{ssh_alias}} "docker logs --tail 1000 -f {{container}}"

# Deploy and tail logs
deploy-logs-masaya:
    @just deploy-masaya
    @just logs-masaya

# Start the masaya service (runs a new container from the existing image)
start-masaya:
    @echo "Starting Taikoscope masaya service..."
    @just stop-container
    @just run-container
    @echo "Taikoscope masaya service started."

# Stop and remove the masaya service
stop-masaya:
    @just stop-container

# Set log level to debug on server and restart the service
debug-log-masaya:
    @just set-log-level debug

# Set log level to info on server and restart the service
info-log-masaya:
    @just set-log-level info

# Search in logs for a specific term
search-logs-masaya term:
    ssh {{ssh_alias}} "docker logs {{container}} | grep -i \"{{term}}\""

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

deploy-api-masaya:
    @echo "Deploying API server via SSH alias '{{ssh_alias}}'"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)
    ssh {{ssh_alias}} "mkdir -p {{remote_dir}}"
    rsync -av --exclude target --exclude .git --exclude dashboard . {{ssh_alias}}:{{remote_dir}}
    @echo "Building API server on {{ssh_alias}} (path: {{remote_dir}})"
    ssh {{ssh_alias}} "cd {{remote_dir}} && docker buildx build --load -f Dockerfile.api -t {{api_container}} ."
    @just start-api-masaya

start-api-masaya:
    @echo "Starting API server..."
    @just stop-api-container
    @just run-api-container
    @echo "API server started."

logs-api-masaya:
    ssh {{ssh_alias}} "docker logs --tail 1000 -f {{api_container}}"

# Deploy and tail logs for the API server
deploy-logs-api-masaya:
    @just deploy-api-masaya
    @just logs-api-masaya

status-api-masaya:
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

# build and push the ingestor docker image with the given tag for the given platforms
build-ingestor tag='latest' platform='linux/amd64,linux/arm64':
    docker buildx build \
        --label "org.opencontainers.image.commit=$(git rev-parse --short HEAD)" \
        --platform {{platform}} \
        --file Dockerfile.ingestor \
        --tag ghcr.io/chainbound/taikoscope-ingestor:{{tag}} \
        --push .


# build and push the docker image with the given tag for the given platforms
build-processor tag='latest' platform='linux/amd64,linux/arm64':
    docker buildx build \
        --label "org.opencontainers.image.commit=$(git rev-parse --short HEAD)" \
        --platform {{platform}} \
        --file Dockerfile.processor \
        --tag ghcr.io/chainbound/taikoscope-processor:{{tag}} \
        --push .

# build and push the api docker image with the given tag for the given platforms
build-api tag='latest' platform='linux/amd64,linux/arm64':
    docker buildx build \
        --label "org.opencontainers.image.commit=$(git rev-parse --short HEAD)" \
        --platform {{platform}} \
        --file Dockerfile.api \
        --tag ghcr.io/chainbound/taikoscope-api:{{tag}} \
        --push .

# build and push both taikoscope and taikoscope-api docker images
build-all tag='latest' platform='linux/amd64,linux/arm64':
    @echo "Building taikoscope images..."
    docker buildx build \
        --label "org.opencontainers.image.commit=$(git rev-parse --short HEAD)" \
        --platform {{platform}} \
        --file Dockerfile.ingestor \
        --tag ghcr.io/chainbound/taikoscope-ingestor:{{tag}} \
        --push .
    docker buildx build \
        --label "org.opencontainers.image.commit=$(git rev-parse --short HEAD)" \
        --platform {{platform}} \
        --file Dockerfile.processor \
        --tag ghcr.io/chainbound/taikoscope-processor:{{tag}} \
        --push .
    @echo "Building taikoscope-api image..."
    docker buildx build \
        --label "org.opencontainers.image.commit=$(git rev-parse --short HEAD)" \
        --platform {{platform}} \
        --file Dockerfile.api \
        --tag ghcr.io/chainbound/taikoscope-api:{{tag}} \
        --push .
