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

# deploy Taikoscope via SSH alias 'taikoscope'
deploy-remote-hekla:
    @echo "Deploying Taikoscope via SSH alias 'taikoscope'"
    @just test || (echo "Tests failed, aborting deployment" && exit 1)

    # Check if "masaya.env" exists. if not, exit with error
    test -f masaya.env || (echo "No masaya.env file found. Exiting." && exit 1)

    # Ensure remote directory exists
    ssh taikoscope "mkdir -p ~/hekla/taikoscope"

    # Copy the project via SSH alias 'taikoscope'
    rsync -av --exclude target --exclude .git . taikoscope:~/hekla/taikoscope

    # Build the docker image via SSH alias 'taikoscope'
    @echo "Building Taikoscope on taikoscope (path: ~/hekla/taikoscope)"
    ssh taikoscope "cd ~/hekla/taikoscope && docker buildx build --load -t taikoscope-hekla ."

    # Stop existing container if running
    ssh taikoscope "docker stop taikoscope-hekla || true"
    ssh taikoscope "docker rm taikoscope-hekla || true"

    # Start new container with environment variables
    ssh taikoscope "docker run -d \
        --name taikoscope-hekla \
        --restart unless-stopped \
        --env-file ~/hekla/taikoscope/masaya.env \
        -p 48100:3000 \
        taikoscope-hekla"

# Check the status of the service
status-remote-hekla:
    ssh taikoscope "docker ps -f name=taikoscope-hekla"

# View the logs of the service
logs-remote-hekla:
    ssh taikoscope "docker logs -f taikoscope-hekla"

# Deploy and tail logs
deploy-logs-remote-hekla:
    @just deploy-remote-hekla
    @just logs-remote-hekla

# Start the remote Hekla service (runs a new container from the existing image)
start-remote-hekla:
    @echo "Starting Taikoscope Hekla service on remote..."
    # The image 'taikoscope-hekla' is assumed to exist on the remote.
    # First, ensure any container with the name 'taikoscope-hekla' is stopped and removed.
    ssh taikoscope "docker stop taikoscope-hekla || true"
    ssh taikoscope "docker rm taikoscope-hekla || true"
    # Then, run a new container.
    ssh taikoscope "docker run -d \
        --name taikoscope-hekla \
        --restart unless-stopped \
        --env-file ~/hekla/taikoscope/masaya.env \
        -p 48100:3000 \
        taikoscope-hekla"
    @echo "Taikoscope Hekla service started."

# Stop and remove the remote Hekla service
stop-remote-hekla:
    ssh taikoscope "docker stop taikoscope-hekla || true"
    ssh taikoscope "docker rm taikoscope-hekla || true"
