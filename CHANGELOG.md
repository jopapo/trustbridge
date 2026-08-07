# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project follows Semantic Versioning.

## [Unreleased]

### Added

- Unified `apply` command with scope orchestration (`runtime,containers,images`) and continuous mode (`--watch`).
- Runtime target support for `rancher-desktop` and `colima`.
- Auto runtime target mode (`--target auto`) with tolerant availability handling.
- Running container patch flow with root execution and trust update detection.
- Local image patch flow with derived image tag strategy (`-tb-<hash>`).
- Incremental sync state using bundle hash tracking for containers/images.
- Corporate-focused default certificate filtering with keyword overrides.
- Optional orchestrator inclusion flag for workload/image patching.
- Auto-install bootstrap for missing `ca-certificates` tooling in targets.
- CI pipeline and release workflow for GitHub Actions.

### Changed

- `apply` default behavior now covers runtime + containers + images.
- State/config storage moved to OS data directories by default, with dev-local mode.
- Runtime removal policy constrained to state-managed fingerprints.

### Documentation

- Architecture documentation updated for current implementation.
- Added ADR-0004 through ADR-0008 for major architectural decisions.
- README updated with current CLI behavior and runtime/patch notes.

## [0.1.0] - 2026-08-07

### Added

- Initial scaffold for TrustBridge CLI.
- Source provider for `macos-keychain`.
- Core domain model, sync plan engine, and command structure.
- Documentation baseline (architecture, ADRs, roadmap, threat model).
