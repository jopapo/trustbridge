# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project follows Semantic Versioning.

## [0.2.1](https://github.com/jopapo/trustbridge/compare/v0.2.0...v0.2.1) (2026-08-10)


### Bug Fixes

* update release workflow to trigger on published releases and ensure correct tag handling ([834ad8f](https://github.com/jopapo/trustbridge/commit/834ad8ffc2cf4ff8d4459f3151c431567906fc8c))

## [0.2.0](https://github.com/jopapo/trustbridge/compare/v0.1.0...v0.2.0) (2026-08-10)


### Features

* add auto target mode for applying patches and include orchestrator/system workloads ([06199bb](https://github.com/jopapo/trustbridge/commit/06199bb8abf72687f67201f05d4908fd7fc636ce))
* add changelog, update contributing guidelines, and document architectural decisions ([07a01f6](https://github.com/jopapo/trustbridge/commit/07a01f6e6a660a88b2fe58d4f0c7f12a1bcbe813))
* add Colima and Rancher Desktop target providers for certificate management ([fcb13d5](https://github.com/jopapo/trustbridge/commit/fcb13d5f86a2080d32b010b93e278c7699507559))
* add container and image patching capabilities with interactive prompts and continuous sync options ([9b0e743](https://github.com/jopapo/trustbridge/commit/9b0e743a72ab98b396df8a20dd26bb115de2205e))
* add dry-run mode indication and enhance error messaging for unavailable runtime targets ([0160fec](https://github.com/jopapo/trustbridge/commit/0160fec7b5207d5877fe1eee19fc5ac7440579a4))
* add release process documentation and checklist for TrustBridge ([935614b](https://github.com/jopapo/trustbridge/commit/935614b148af44db744156eec63daddfe252743c))
* add release process documentation and checklist for TrustBridge ([ad97ac0](https://github.com/jopapo/trustbridge/commit/ad97ac003ae96e7185873c74dc8d0f490a4181a7))
* enhance scanning and applying of certificates with filtering options ([98b6c92](https://github.com/jopapo/trustbridge/commit/98b6c929bc0866209db92b9d7fa590ea33c858ca))
* implement automated release process with Release Please and add PR title validation ([bb24328](https://github.com/jopapo/trustbridge/commit/bb2432824bb5e572fadd7144294b0bc6e248e9ec))
* implement CI and release workflows, add support for Colima target and CA update tool checks ([c32b9aa](https://github.com/jopapo/trustbridge/commit/c32b9aa86e819f9ea829746652ef56c07f40247a))
* initialize TrustBridge project with macOS Keychain source and Rancher Desktop target ([19ccbec](https://github.com/jopapo/trustbridge/commit/19ccbec3381bcf51324fa50466669360c1d062de))


### Bug Fixes

* update debug module layout step to use PowerShell for improved file listing ([cdad1fa](https://github.com/jopapo/trustbridge/commit/cdad1fafca9d1ffec1e609527e71c9f7d08dd55b))

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
