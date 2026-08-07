# ADR-0007: Incremental Sync with Bundle Hash State Tracking

- Status: Accepted
- Date: 2026-08-07

## Context

Repeated full patch cycles are expensive and noisy in watch mode, especially for many containers/images.

## Decision

Track bundle-level synchronization state:

- compute bundle hash from selected certificate fingerprints
- persist `last_bundle_hash`
- persist container/image hash maps (`target -> bundle_hash`)
- skip targets already synced to current bundle hash

## Consequences

### Positive

- reduced repeated work
- faster watch cycles
- lower operational noise and risk

### Trade-offs

- larger state model and migration considerations
- possible stale state handling requirements in future
