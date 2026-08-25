# TrustBridge (`tbridge`)

![CI](https://github.com/jopapo/trustbridge/actions/workflows/ci.yml/badge.svg)
![Release Please](https://github.com/jopapo/trustbridge/actions/workflows/release-please.yml/badge.svg)
![GitHub tag](https://img.shields.io/github/v/tag/jopapo/trustbridge)
![License](https://img.shields.io/github/license/jopapo/trustbridge)

TrustBridge is a CLI that syncs trusted host certificates into local container runtimes.

## Who is this for?

If your company uses TLS inspection/proxy (self-signed or corporate CAs), your host usually trusts those certs, but local runtimes often do not. That leads to TLS errors in pulls, builds, and HTTPS calls inside containers.

`tbridge` fixes this at the host/runtime boundary.

## Important note (self-signed)

> Run `tbridge` on the **host that already has the self-signed/corporate certificates trusted**.
>
> If you run it on a different machine/VM/WSL distro without the same trust store, `scan` won't find the right certs and `apply` won't produce the expected result.

## Supported providers

- **Source (host):** `macos-keychain`, `windows-certstore`
- **Targets:** `rancher-desktop`, `colima`, `docker-desktop`, `wsl`

## Quick start

```bash
# 1) Discover trusted certs from host
cargo run -- scan

# 2) Preview what would change
cargo run -- plan

# 3) Apply sync
cargo run -- apply

# 4) Verify trust against a host
cargo run -- verify --host registry.corp.local:443
```

## Common usage

```bash
# Select only specific roots by keyword
cargo run -- plan --only-keywords netskope,inbev

# Dry-run apply
cargo run -- apply --dry-run

# Force a target
cargo run -- apply --target colima
cargo run -- apply --target rancher-desktop
cargo run -- apply --target docker-desktop
cargo run -- apply --target wsl

# Continuous sync
cargo run -- apply --watch --interval-secs 30
```

## Runtime notes

- **Windows Certificate Store:** reads `Cert:\LocalMachine\Root` and `Cert:\CurrentUser\Root` via PowerShell.
- **Rancher Desktop / Colima:** managed certs are written inside the VM and system CA store is updated.
- **Docker Desktop (Windows):** targets the `docker-desktop` WSL2 distro.
- **WSL target:** runs against current distro or one set with `TBRIDGE_WSL_DISTRO=<name>`.

## Install artifacts

Release binaries are published in GitHub Releases.

- Windows: `tbridge-windows-x86_64-vX.Y.Z.zip`
- Linux (glibc): `tbridge-linux-gnu-x86_64-vX.Y.Z.tar.gz`
- Linux (musl): `tbridge-linux-musl-x86_64-vX.Y.Z.tar.gz`
- macOS: `tbridge-macos-x86_64-vX.Y.Z.tar.gz`

For development details, project structure, and contribution workflow, see `CONTRIBUTING.md`.

## License

MIT — see `LICENSE`.
