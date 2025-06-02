# Taikoscope 🔭

Taikoscope is a collection of tools for monitoring the Taiko network. It pulls
data from on‑chain sources, stores the results in ClickHouse and exposes them
through a thin HTTP API consumed by the dashboard frontend.

## Table of Contents

1. [Requirements](#requirements)
2. [Quick Start](#quick-start)
3. [Environment](#environment)
4. [Development](#development)
5. [Deployment](#deployment)
6. [License](#license)

## Requirements

- [Rust](https://www.rust-lang.org/) (2024 edition)
- [just](https://github.com/casey/just)
- A running ClickHouse instance
- Access to L1 and L2 RPC endpoints
- Node.js (for the dashboard)

## Quick Start

1. Clone the repository and install dependencies.
2. Copy `dev.env` and adjust the values for your setup or provide your own env
   file via the `ENV_FILE` variable.
3. (Optional) Start ClickHouse and the dashboard via Docker Compose:

   ```bash
   docker compose up
   ```

4. Start the extractor and API server:

   ```bash
   just dev         # runs the extractor/driver
   just dev-api     # runs the HTTP API
   ```

5. Start the dashboard (optional if not using Docker Compose):

   ```bash
   just dev-dashboard
   ```

The API is now available on `http://localhost:3000` and the dashboard on
`http://localhost:5173` by default.

## Environment

All configuration is provided via environment variables. The most relevant
variables are shown below. See [`crates/config`](crates/config) for the full
list.

```text
CLICKHOUSE_URL=<http://localhost:8123>
CLICKHOUSE_DB=taikoscope
L1_RPC_URL=<l1-endpoint>
L2_RPC_URL=<l2-endpoint>
TAIKO_INBOX_ADDRESS=<0x...>
TAIKO_PRECONF_WHITELIST_ADDRESS=<0x...>
TAIKO_WRAPPER_ADDRESS=<0x...>
API_HOST=127.0.0.1
API_PORT=3000
```

## Development

Formatting, linting and tests can be run via `just`:

```bash
just fmt      # format the code
just lint     # run clippy
just test     # run the test suite
just ci       # runs fmt, lint and test
```

## Deployment

Deployment scripts use `ssh` and `docker` to build the images remotely.
Create an entry in your `~/.ssh/config` (for example named `taiko`) and then run:

```bash
just deploy-remote-hekla        # deploy the extractor/driver
just deploy-api-remote-hekla    # deploy the API server
```

## License

Licensed under the MIT license. See [`LICENSE`](LICENSE) for details.
