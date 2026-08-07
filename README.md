# TrustBridge (`tbridge`)

TrustBridge is an open-source CLI to sync trusted host certificates into local container runtimes.

Initial focus (MVP): **macOS Keychain -> Rancher Desktop**.

## Why

Corporate environments often use TLS interception/proxy stacks (Netskope, Zscaler, Blue Coat, Palo Alto, etc.).
Those CAs are trusted on the host, but not always inside local container runtimes, causing TLS errors in developer workflows.

TrustBridge aims to solve this once at the host/runtime boundary, instead of per-project patches.

## Current Status

- Project stage: `v0.1.0` scaffold
- Source provider: `macos-keychain` (implemented)
- Target provider: `rancher-desktop` (safe stub for incremental implementation)
- Commands: `scan`, `plan`, `apply`, `verify`

## Design Principles

- **Security-first**: sync only public certs, never private keys
- **Idempotent**: desired-state synchronization with diff/plan
- **Auditable**: explicit plan before apply
- **Extensible**: provider-based architecture for sources/targets
- **Cross-platform roadmap**: macOS first, then Windows/Linux

## CLI

```bash
# Scan trusted certs from macOS keychain
cargo run -- scan

# Scan self-signed certs including public/OS roots
cargo run -- scan --include-public-roots

# Scan only specific corporate roots
cargo run -- scan --only-keywords netskope,inbev

# Scan all certs (disable default self-signed filter)
cargo run -- scan --all

# Show sync plan between source and target
cargo run -- plan

# Plan using only specific corporate roots
cargo run -- plan --only-keywords netskope,inbev

# Execute sync (real apply by default)
cargo run -- apply

# Dry-run using only specific corporate roots
cargo run -- apply --dry-run --only-keywords netskope,inbev

# Apply for real using only specific corporate roots
cargo run -- apply --only-keywords netskope,inbev

# Verify target trust behavior (stub for now)
cargo run -- verify --host registry.corp.local:443
```

## Architecture Overview

Pipeline:

1. Discover certificates from source provider
2. Normalize certificates and compute SHA-256 fingerprints
3. Diff source vs target trust state
4. Build sync plan (`to_add`, `to_remove`)
5. Apply sync plan (or dry-run)
6. Verify trust behavior

Main modules:

- `src/core`: domain models and sync engine
- `src/providers/source`: host trust source providers
- `src/providers/target`: runtime target providers
- `src/commands`: CLI command handlers

## Repository Layout

- `src/main.rs`: CLI entrypoint
- `src/cli.rs`: command/args definitions
- `src/core/`: engine, plan, state, certificate model
- `src/providers/source/macos_keychain.rs`: source provider
- `src/providers/target/rancher_desktop.rs`: target provider (stub)
- `docs/`: architecture, roadmap, ADRs, security notes
- `examples/`: sample configuration

## Documentation Index

- `docs/architecture.md`
- `docs/roadmap.md`
- `docs/decision-log.md`
- `docs/decision-process.md`
- `docs/adr/ADR-0001-rust-and-provider-architecture.md`
- `docs/adr/ADR-0002-macos-to-rancher-desktop-mvp.md`
- `docs/adr/ADR-0003-safety-and-rollout-strategy.md`
- `docs/threat-model.md`
- `CONTRIBUTING.md`

## Build & Validate

```bash
cargo fmt
cargo check
cargo test
```

If dependency fetch fails in restricted environments, run in an environment with crates.io access first.

## State & Config Paths

- Installed mode (default):
  - macOS: `~/Library/Application Support/trustbridge/`
  - Linux: `$XDG_DATA_HOME/trustbridge/` (fallback `~/.local/share/trustbridge/`)
  - Windows: `%APPDATA%/trustbridge/`
- Dev-local mode is auto-enabled when running via `cargo run`.
- You can also force dev-local with `TBRIDGE_DEV_LOCAL=1` to use `.tbridge/` in the project directory.

## License

MIT
