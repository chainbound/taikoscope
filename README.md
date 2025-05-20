# Taikoscope 🔭

## Project Description

Taikoscope is a unified dashboard developed by Chainbound to provide real-time, granular insights into the Taiko network. The dashboard focuses on based sequencing, offering performance metrics and economic data to support monitoring, research, and community transparency. It aggregates uptime, sequencer performance, and gateway economics into a single, accessible interface, supporting both operational needs and ecosystem research.

## Requirements
- Rust
- Anvil
- [just](https://github.com/casey/just)
- L1 RPC
- L2 RPC
- ClickHouse instance (local or remote)

## Deploy

Run locally
```
just dev
```

Or deploy to cloud

First, create SSH config alias

```
Host taiko
  HostName machine
  User username
  IdentityFile ~/.ssh/id_ed25519
```

Then run deploy script
```
just deploy-remote-hekla
```
