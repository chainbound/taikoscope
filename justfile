set shell := ["bash", "-cu"]
set dotenv-load := true

# common configuration

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
