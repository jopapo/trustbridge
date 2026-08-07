# ADR-0004: Default Corporate CA Selection and Filter Overrides

- Status: Accepted
- Date: 2026-08-07

## Context

Raw host trust stores include many public/OS roots that are not relevant for enterprise proxy trust remediation. Applying all roots by default creates noise, longer patch times, and risk of unintended trust propagation.

## Decision

Use a focused default certificate selection strategy:

- self-signed certificate focus
- exclude likely public/OS roots by default
- allow explicit operator overrides via:
  - `--include-public-roots`
  - `--only-keywords`
  - `--exclude-keywords`

## Consequences

### Positive

- defaults align with corporate CA use cases (Netskope/InBev-like roots)
- smaller and safer trust mutation set
- improved operator control for edge cases

### Trade-offs

- heuristic root exclusion may need tuning over time
- some environments require explicit flags for broader trust sets
