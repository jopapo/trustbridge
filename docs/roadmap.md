# Roadmap

## v0.1 - Foundation (reset baseline)

- Rust CLI foundation
- Source provider: `macos-keychain`
- Runtime targets: `rancher-desktop`, `colima`, and `auto` target mode
- Unified `apply` scopes: `runtime`, `containers`, `images`
- Corporate-focused default certificate filtering
- Incremental sync state (`bundle_hash` + per-target hashes)
- Release automation (`Release Please` + multi-platform binaries)

## v0.2 - Runtime & Workload Hardening

- Improve runtime detection and fallback diagnostics
- Strengthen rollback and partial-failure handling for all scopes
- Better support for immutable/read-only containers and images
- Add scoped status/report command for sync visibility
- Harden release governance (squash + semantic title policy enforcement)

## v0.3 - Policy & Configuration

- Policy profiles (corp-focused, broad trust, strict)
- Config-driven defaults (keywords/scopes/targets/watch interval)
- Fingerprint allowlist/denylist and expiration guardrails
- Dry-run output in machine-readable report format

## v0.4 - Kubernetes-native Integration

- Kubernetes workload patch mode (selector-based)
- Optional manifest patch strategy (ConfigMap + volume mounts)
- Namespace-aware reporting and controlled rollout modes

## v0.5 - Cross-platform Source Expansion

- Windows certificate store source provider
- Linux trust store source provider
- Cross-platform parity tests and docs

## v1.0 - Stable OSS Baseline

- Stable provider contracts and migration guarantees
- End-to-end test matrix and reliability SLAs
- Production-ready release packaging and operational docs
- Signed and notarized macOS binaries (remove local unblock workaround)
