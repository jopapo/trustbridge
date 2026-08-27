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
tbridge scan

# 2) Preview what would change
tbridge plan

# 3) Apply sync
tbridge apply

# 4) Verify trust against a host
tbridge verify --host registry.corp.local:443
```

## Common usage

```bash
# Select only specific roots by keyword
tbridge plan --only-keywords netskope,inbev

# Use the corporate profile preset
tbridge scan --profile corp
tbridge apply --dry-run --profile corp --target wsl --scope runtime

# Dry-run apply
tbridge apply --dry-run

# Force a target
tbridge apply --target colima
tbridge apply --target rancher-desktop
tbridge apply --target docker-desktop
tbridge apply --target wsl

# Continuous sync
tbridge apply --watch --interval-secs 30
```

## Opening by double-click

If you open `tbridge`/`tbridge.exe` by double-click, TrustBridge shows a quick usage guide
with command examples.

Recommended usage is from terminal/PowerShell:

```bash
tbridge scan
tbridge plan
tbridge apply --dry-run
tbridge apply
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
