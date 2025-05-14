set shell := ["bash", "-cu"]
set dotenv-load := true

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

# deploy Taikoscope to remotesmol
deploy-remote-hekla:
    @echo "Deploying Taikoscope on remotesmol"

    # Check if "masaya.env" exists. if not, exit with error
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)

    # Ensure remote directory exists
    ssh shared@remotesmol "mkdir -p /home/shared/hekla/taikoscope"

    # Copy the project to remotesmol
    rsync -av --exclude target --exclude .git . shared@remotesmol:/home/shared/hekla/taikoscope

    # Build the docker image on remotesmol
    @echo "Building Taikoscope on remotesmol (path: /home/shared/hekla/taikoscope)"
    ssh shared@remotesmol "cd ~/hekla/taikoscope && docker buildx build --load -t taikoscope-hekla ."

    # Stop existing container if running
    ssh shared@remotesmol "docker stop taikoscope-hekla || true"
    ssh shared@remotesmol "docker rm taikoscope-hekla || true"

    # Start new container with environment variables
    ssh shared@remotesmol "docker run -d \
        --name taikoscope-hekla \
        --restart unless-stopped \
        --env-file ~/hekla/taikoscope/masaya.env \
        -p 3000:3000 \
        taikoscope-hekla"

# Check the status of the service
status-remote-hekla:
    ssh shared@remotesmol "docker ps -f name=taikoscope-hekla"

# View the logs of the service
logs-remote-hekla:
    ssh shared@remotesmol "docker logs -f taikoscope-hekla"

# Deploy and tail logs
deploy-logs-remote-hekla:
    @just deploy-remote-hekla
    @just logs-remote-hekla

# Stop the service
stop-remote-hekla:
    ssh shared@remotesmol "docker stop taikoscope-hekla"
    ssh shared@remotesmol "docker rm taikoscope-hekla"
