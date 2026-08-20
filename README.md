# TrustBridge (`tbridge`)

![CI](https://github.com/jopapo/trustbridge/actions/workflows/ci.yml/badge.svg)
![Release Please](https://github.com/jopapo/trustbridge/actions/workflows/release-please.yml/badge.svg)
![GitHub tag](https://img.shields.io/github/v/tag/jopapo/trustbridge)
![License](https://img.shields.io/github/license/jopapo/trustbridge)

TrustBridge is an open-source CLI to sync trusted host certificates into local container runtimes.

Supported today: **macOS Keychain -> Rancher Desktop / Colima** and **Windows Certificate Store -> Rancher Desktop / Docker Desktop / WSL**.

## Why

Corporate environments often use TLS interception/proxy stacks (Netskope, Zscaler, Blue Coat, Palo Alto, etc.).
Those CAs are trusted on the host, but not always inside local container runtimes, causing TLS errors in developer workflows.

TrustBridge aims to solve this once at the host/runtime boundary, instead of per-project patches.

## Current Status

- Project stage: `v0.2.3`
- Source providers: `macos-keychain`, `windows-certstore` (auto-selected based on host OS)
- Target providers: `rancher-desktop`, `colima`, `docker-desktop`, `wsl`
- Commands: `scan`, `plan`, `apply`, `verify`

## Design Principles

- **Security-first**: sync only public certs, never private keys
- **Idempotent**: desired-state synchronization with diff/plan
- **Auditable**: explicit plan before apply
- **Extensible**: provider-based architecture for sources/targets
- **Cross-platform**: macOS (Keychain) and Windows/WSL (Certificate Store) sources implemented; Linux source planned

## CLI

```bash
# Scan trusted certs from the OS-appropriate source (macos-keychain or windows-certstore)
cargo run -- scan

# Scan self-signed certs including public/OS roots
cargo run -- scan --include-public-roots

# Scan only specific corporate roots
cargo run -- scan --only-keywords netskope,inbev

# Scan all certs (disable default self-signed filter)
cargo run -- scan --all

# Scan the Windows Certificate Store explicitly (also the default on Windows/WSL)
cargo run -- scan --source windows-certstore

# Show sync plan between source and target
cargo run -- plan

# Plan using only specific corporate roots
cargo run -- plan --only-keywords netskope,inbev

# Execute sync (real apply by default)
cargo run -- apply

# Auto target mode checks compatible runtimes for the current OS
# (macOS: rancher-desktop, colima; Windows/WSL: rancher-desktop, docker-desktop, wsl)
# and applies to available ones, tolerating unavailable targets
cargo run -- apply --target auto

# Execute sync against Colima
cargo run -- apply --target colima

# Execute sync against Docker Desktop's WSL2 backend (Windows only)
cargo run -- apply --target docker-desktop

# Execute sync against a WSL distro directly
cargo run -- apply --target wsl

# Dry-run using only specific corporate roots
cargo run -- apply --dry-run --only-keywords netskope,inbev

# Apply for real using only specific corporate roots
cargo run -- apply --only-keywords netskope,inbev

# Patch specific containers with confirmation prompts
cargo run -- apply --containers technology-samples-app-1 --interactive

# Scope control (default: runtime,containers,images)
cargo run -- apply --scope runtime,containers

# Include orchestrator/system workloads and images (default is user-focus)
cargo run -- apply --include-orchestrator

# Keep sync running continuously
cargo run -- apply --watch --interval-secs 30

# Verify target trust behavior
cargo run -- verify --host registry.corp.local:443
```

Notes for Windows Certificate Store scan:

- Reads `Cert:\LocalMachine\Root` and `Cert:\CurrentUser\Root` via PowerShell (`powershell.exe`/`pwsh`).
- Works natively on Windows and from inside WSL (via Windows interop).
- Auto-selected as the default `--source` on Windows and WSL; use `--source windows-certstore` to force it elsewhere.

Notes for Rancher Desktop apply:

- Uses `limactl shell` (instance `0` by default) on macOS, or the `rancher-desktop` WSL2 distro via `wsl.exe` on Windows/WSL.
- Writes managed certs into `/usr/local/share/ca-certificates/tbridge/` inside the VM.
- Runs `update-ca-certificates` (or `update-ca-trust extract` when available).
- Override instance with `TBRIDGE_RD_INSTANCE=<name>`.

Notes for Colima apply:

- Uses `limactl shell` with default instance `colima`.
- Override instance with `TBRIDGE_COLIMA_INSTANCE=<name>`.
- macOS only.

Notes for Docker Desktop apply (Windows only):

- Reaches Docker Desktop's own `docker-desktop` WSL2 distro via `wsl.exe`.
- Writes managed certs into `/usr/local/share/ca-certificates/tbridge/` inside that distro.
- Override the distro name with `TBRIDGE_DOCKER_DESKTOP_INSTANCE=<name>`.

Notes for WSL apply:

- Targets the WSL distro tbridge is running in, or another distro via `wsl.exe`.
- Set `TBRIDGE_WSL_DISTRO=<name>` to target a specific distro (including from Windows).
- Without the env var, requires running tbridge directly inside the target WSL distro.

Notes for container/image patch (inside `apply`):

- Uses `docker exec -u 0` to patch running containers.
- By default, focuses on user containers/images and skips orchestrator/system ones.
- Use `--include-orchestrator` to include k8s/system workloads and images.
- Detects `update-ca-certificates` or `update-ca-trust extract` automatically.
- If CA tooling is missing, attempts to install `ca-certificates` as root via distro package manager.
- Copies selected certs as `*.crt` and runs system CA update inside each container.
- Patches local images too (default `--images-mode user`, configurable with `user|all|none`).
- Commits patched image variants with suffix `-tb-<bundle_hash>`.
- `--dry-run` prints detected strategy without modifying targets.
- Incremental sync uses persisted state to skip containers/images already patched for the current bundle hash.
- `--watch` runs repeated sync cycles and keeps retrying even if a cycle fails.

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
- `src/providers/source/macos_keychain.rs`: macOS Keychain source provider
- `src/providers/source/windows_certstore.rs`: Windows Certificate Store source provider
- `src/providers/target/`: runtime target providers (`rancher-desktop`, `colima`, `docker-desktop`, `wsl`)
- `src/providers/target/vm_backend.rs`: shared shell transport (Lima, `rdctl`, `wsl.exe`, local) for VM-backed targets
- `docs/`: architecture, roadmap, ADRs, security notes
- `examples/`: sample configuration

## Documentation Index

- `docs/architecture.md`
- `docs/roadmap.md`
- `docs/decision-log.md`
- `docs/decision-process.md`
- `docs/release-process.md`
- `docs/adr/ADR-0001-rust-and-provider-architecture.md`
- `docs/adr/ADR-0002-macos-to-rancher-desktop-mvp.md`
- `docs/adr/ADR-0003-safety-and-rollout-strategy.md`
- `docs/adr/ADR-0004-filtering-and-corporate-ca-selection.md`
- `docs/adr/ADR-0005-unified-apply-scopes-and-continuous-sync.md`
- `docs/adr/ADR-0006-runtime-target-auto-detection.md`
- `docs/adr/ADR-0007-incremental-state-and-bundle-hash.md`
- `docs/adr/ADR-0008-ca-tooling-bootstrap.md`
- `docs/threat-model.md`
- `CONTRIBUTING.md`
- `CHANGELOG.md`

## Build & Validate

```bash
cargo fmt
cargo check
cargo test
```

If dependency fetch fails in restricted environments, run in an environment with crates.io access first.

## Versioning & Releases

- Semantic versioning is automated via Release Please from commits merged into `main`.
- Stable tags follow `vX.Y.Z`; pre-releases can use `vX.Y.Z-alpha.N`, `-beta.N`, or `-rc.N`.
- Tag-based binary publication remains handled by GitHub Actions release workflow.
- Recommended governance: semantic PR titles + squash-only merges on `main`.
- Full checklist: `docs/release-process.md`.

## macOS Security Note (Unsigned Builds)

Current release binaries are not yet Apple code-signed/notarized. On macOS, Gatekeeper may block execution of downloaded binaries.

Use the helper script to remove the quarantine attribute for local usage:

```bash
./scripts/macos-unblock.sh ./tbridge-macos-x86_64
./tbridge-macos-x86_64 --help
```

When downloading from GitHub Releases, the macOS `.tar.gz` bundle already includes
`macos-unblock.sh` and a short `README-macos.txt` with these same steps.

If you extracted a different filename/path, pass that path to the script.

## State & Config Paths

- Installed mode (default):
  - macOS: `~/Library/Application Support/trustbridge/`
  - Linux: `$XDG_DATA_HOME/trustbridge/` (fallback `~/.local/share/trustbridge/`)
  - Windows: `%APPDATA%/trustbridge/`
- Dev-local mode is auto-enabled when running via `cargo run`.
- You can also force dev-local with `TBRIDGE_DEV_LOCAL=1` to use `.tbridge/` in the project directory.

## License

MIT — see [LICENSE](LICENSE).
