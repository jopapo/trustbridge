# Architecture

## Objective

Provide a secure, auditable, and extensible way to sync host-trusted CA certificates into local developer runtimes, running containers, and local images.

## Sync Lifecycle

1. `scan`: read and normalize trusted certificates from a source
2. `plan`: compare filtered source set vs runtime target state
3. `apply`: execute sync across selected scopes (`runtime`, `containers`, `images`)
4. `verify`: validate trust behavior against endpoints/runtime checks
5. `watch` (via `apply --watch`): run repeated incremental sync cycles

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

- `StateSnapshot`
  - `last_apply_at`
  - `applied_fingerprints`
  - `last_bundle_hash`
  - `container_bundle_hashes`
  - `image_bundle_hashes`

## Certificate Selection Model

Default certificate selection is security-focused and developer-focused:

- include self-signed certs
- exclude likely public/OS roots
- allow explicit overrides:
  - `--include-public-roots`
  - `--only-keywords`
  - `--exclude-keywords`

This keeps operational trust focused on likely corporate CAs (for example proxy/intercept roots).

## Apply Scopes

`apply` can act on one or more scopes:

- `runtime`: patch runtime VM trust store (Rancher Desktop / Colima)
- `containers`: patch running Docker containers in-place
- `images`: patch local Docker images by creating derived patched image tags

Default scope:

- `runtime,containers,images`

## Runtime Providers

### Source Provider

- `macos-keychain` (current)

### Target Providers

- `rancher-desktop`
- `colima`
- `auto` target resolution for apply (tries compatible targets and proceeds with available ones)

Runtime patch flow:

1. resolve runtime target(s)
2. compute runtime plan
3. apply cert add/remove into managed directory
4. ensure CA tooling exists (install `ca-certificates` when missing)
5. run trust update command
6. rollback changed files on runtime apply failure

## Workload and Image Patching

### Running Containers

- discover running containers (or explicit `--containers`)
- by default, skip orchestrator/system containers
- optional `--include-orchestrator` includes system/k8s containers
- attempt CA tooling install when absent
- write certs and run trust update command as root (`docker exec -u 0`)

### Local Images

- select images by mode (`user|all|none`, default `user`)
- optionally include orchestrator/system images
- patch in temporary container and commit derived image
- derived tag suffix: `-tb-<bundle_hash_prefix>`
- attempt CA tooling install when absent

## Incremental Sync

Bundle hash is computed from selected certificate fingerprints.

State tracks per-target hash application:

- if container/image already has current bundle hash, skip patching
- in watch mode, retry periodically and patch only deltas when possible

## Paths and Persistence

State/config paths use host OS data directories in installed mode:

- macOS: `~/Library/Application Support/trustbridge/`
- Linux: `$XDG_DATA_HOME/trustbridge/` (fallback `~/.local/share/trustbridge/`)
- Windows: `%APPDATA%/trustbridge/`

Dev-local mode uses `.tbridge/`:

- auto-enabled under `cargo run`
- or forced with `TBRIDGE_DEV_LOCAL=1`

## Safety Model

- dry-run available for all apply scopes
- runtime remove operations limited to state-managed fingerprints
- explicit logging per target/scope
- tolerant multi-target runtime behavior in `auto` mode
  - continue when one runtime target is unavailable
  - fail runtime scope only when no compatible target is available and no other scopes are selected

## Non-Goals (current)

- patching immutable distroless/scratch workloads without workaround strategies
- automatic cluster manifest mutation (ConfigMap/volume/env injection)
- centralized policy engine for trust governance
