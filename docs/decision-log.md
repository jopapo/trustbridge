# Decision Log

This file summarizes major project decisions with context and rationale.

## 2026-08-07 - Language choice: Rust

- Need: safe systems-level CLI, single binary distribution, good performance.
- Decision: Rust.
- Why: strong type safety, robust error handling, cross-platform ecosystem.

## 2026-08-07 - MVP scope: macOS -> Rancher Desktop

- Need: solve immediate corporate TLS pain for macOS developers using Rancher Desktop.
- Decision: prioritize source=`macos-keychain`, target=`rancher-desktop`.
- Why: narrow scope enables fast validation and community feedback.

## 2026-08-07 - Architecture: provider interfaces

- Need: future support for multiple sources/targets without rewrites.
- Decision: define `SourceProvider` and `TargetProvider` traits.
- Why: keeps core sync logic reusable and target-agnostic.

## 2026-08-07 - Safety-first rollout

- Need: prevent trust misconfiguration and risky default behavior.
- Decision: keep target integration as stub first, dry-run friendly flow.
- Why: build confidence before automating VM truststore mutation.
