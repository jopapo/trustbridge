# ADR-0006: Runtime Target Auto-Detection with Tolerant Availability

- Status: Accepted
- Date: 2026-08-07

## Context

Users may run Rancher Desktop, Colima, both, or neither. Forcing a single explicit target can increase friction and operational failures.

## Decision

Introduce `target=auto` for apply (default), checking compatible runtime targets:

- Rancher Desktop
- Colima

Behavior:

- apply to available runtime targets
- warn (not fail) for unavailable targets
- fail only when runtime scope is requested and none are available (unless non-runtime scopes are also selected)

## Consequences

### Positive

- lower configuration burden
- resilient behavior across local environments
- better compatibility with mixed runtime setups

### Trade-offs

- more complex runtime orchestration logic
- warning-heavy output in partially configured systems
