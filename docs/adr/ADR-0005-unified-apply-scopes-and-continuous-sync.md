# ADR-0005: Unified Apply Across Runtime, Containers, and Images

- Status: Accepted
- Date: 2026-08-07

## Context

Developers use heterogeneous images and cannot always modify Dockerfiles. Trust sync must work across runtime VM, running containers, and local images with minimal operator friction.

## Decision

Consolidate operations into `apply` with scope-based behavior:

- default scopes: `runtime,containers,images`
- optional scope restriction via `--scope`
- continuous mode via `apply --watch`

Also add user-focused defaults:

- patch user workloads/images by default
- include orchestrator/system targets only with `--include-orchestrator`

## Consequences

### Positive

- single command for common trust sync workflows
- better developer ergonomics for frequently changing images
- easier operational adoption and automation

### Trade-offs

- broader default behavior requires careful observability
- additional scope complexity in apply orchestration
