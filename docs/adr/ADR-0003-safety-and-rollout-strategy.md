# ADR-0003: Safety-first Rollout

- Status: Accepted
- Date: 2026-08-07

## Context

Certificate trust changes can break developer environments if applied incorrectly.

## Decision

Start with a conservative rollout:

- explicit scan/plan/apply flow
- `rancher-desktop` target begins as controlled stub
- implement mutation only after observability and rollback design

## Consequences

### Positive

- lowers operational risk
- improves clarity for contributors and maintainers

### Trade-offs

- slower path to full automation
- early adopters need patience during implementation phases
