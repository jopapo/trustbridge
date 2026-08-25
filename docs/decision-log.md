# Decision Log

This file summarizes major project decisions with context and rationale.

## 2026-08-07 - Language choice: Rust

- Need: safe systems-level CLI, single binary distribution, good performance.
- Decision: Rust.
- Why: strong type safety, robust error handling, cross-platform ecosystem.

## 2026-08-07 - MVP scope: macOS -> Rancher Desktop

- Need: solve immediate corporate TLS pain for macOS developers using Rancher Desktop runtime.
- Decision: prioritize source=`macos-keychain`, target=`rancher-desktop`.
- Why: narrow scope enables fast validation and community feedback.

## 2026-08-07 - Architecture: provider interfaces

- Need: future support for multiple sources/targets without rewrites.
- Decision: define `SourceProvider` and `TargetProvider` traits.
- Why: keeps core sync logic reusable and target-agnostic.

## 2026-08-07 - Safety-first rollout

- Need: prevent trust misconfiguration and risky default behavior.
- Decision: explicit scan/plan/apply flow and conservative rollout sequencing.
- Why: build confidence before broad truststore mutation automation.

## 2026-08-07 - ADR-0004 Default corporate CA filtering

- Need: avoid noisy/default syncing of public/OS roots.
- Decision: self-signed + corporate-focused default with keyword/public-root overrides.
- Why: safer and more relevant defaults for enterprise proxy use cases.

## 2026-08-07 - ADR-0005 Unified apply scopes + watch

- Need: one operational command for runtime + workloads + images.
- Decision: `apply` orchestrates all scopes with `--scope` and `--watch`.
- Why: improves developer ergonomics and automation consistency.

## 2026-08-07 - ADR-0006 Runtime auto-target

- Need: support mixed Rancher/Desktop and Colima environments.
- Decision: add `target=auto` and tolerant target availability logic.
- Why: reduces setup friction and avoids hard failures in partial environments.

## 2026-08-07 - ADR-0007 Incremental bundle hash sync

- Need: reduce repetitive patch cycles and watch-mode overhead.
- Decision: persist bundle hash state per container/image target.
- Why: enables efficient incremental synchronization.

## 2026-08-07 - ADR-0008 CA tooling bootstrap

- Need: handle images/targets missing CA update tooling.
- Decision: attempt `ca-certificates` install via supported package managers.
- Why: maximize compatibility across diverse runtimes and base images.

## 2026-08-25 - ADR-0009 Release artifact compatibility + TLS prerequisites

- Need: reduce repeated install failures across Windows/WSL/Linux and corporate TLS interception environments.
- Decision: publish Linux `gnu` and `musl` artifacts, and document artifact-selection + TLS trust prerequisites in core docs/release flow.
- Why: improve cross-machine reliability and lower support friction for non-code environment issues.
