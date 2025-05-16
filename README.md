# Taikoscope 🔭

## Requirements
- Rust
- Anvil
- [just](https://github.com/casey/just)

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
