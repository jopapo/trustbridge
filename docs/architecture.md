# Architecture

## Objective

Provide a secure, auditable, and extensible way to sync host-trusted CA certificates into local container runtimes.

## Sync Lifecycle

1. `scan`: read trusted certificates from a source
2. `plan`: compare source certificates with target truststore state
3. `apply`: execute a synchronization plan
4. `verify`: validate trust behavior against test endpoints

## Domain Model

- `Certificate`
  - `id`
  - `subject`
  - `fingerprint_sha256`
  - `pem`
  - `not_after`

- `SyncPlan`
  - `source_total`
  - `target_total`
  - `to_add`
  - `to_remove`

## Provider Interfaces

### `SourceProvider`

- `name() -> &'static str`
- `scan() -> Result<Vec<Certificate>>`

### `TargetProvider`

- `name() -> &'static str`
- `current_fingerprints() -> Result<Vec<String>>`
- `apply_plan(plan, dry_run) -> Result<()>`
- `verify(host) -> Result<()>`

## Initial Provider Pair

- Source: `macos-keychain`
- Target: `rancher-desktop`

## Rancher Desktop Implementation Plan (next)

1. Detect runtime mode and Lima instance name
2. Build managed bundle from selected source certificates
3. Copy bundle into VM using `limactl shell`/`limactl copy` strategy
4. Install certs under distro-specific CA path
5. Run `update-ca-certificates` (or distro equivalent)
6. Persist applied fingerprints in local state
7. Add rollback on failed update

## State

Local state file:

- `.tbridge/state.json`

Tracks:

- last successful apply timestamp
- applied fingerprints snapshot

## Non-Goals (v0.1)

- Full cross-platform support
- Automatic certificate policy engine
- Runtime-specific deep integration beyond Rancher Desktop
