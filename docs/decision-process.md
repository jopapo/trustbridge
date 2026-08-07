# Decision-Making Process

This document explains how architectural and product decisions are made in TrustBridge.

## Goals

- Make decisions transparent for new contributors
- Preserve rationale beyond chat/session context
- Keep product direction consistent over time

## Process

1. Identify a concrete problem/opportunity
2. Document context and constraints
3. List realistic options
4. Evaluate trade-offs (security, complexity, UX, maintenance)
5. Choose one option and record rationale
6. Publish as ADR in `docs/adr/`
7. Revisit if assumptions change

## Decision Criteria

Primary criteria (in order):

1. Security and operational safety
2. Correctness and deterministic behavior
3. User impact (developer pain reduction)
4. Extensibility for new providers
5. Maintenance burden

## Decision Levels

- **Minor**: code-level implementation detail, no ADR required
- **Significant**: behavior/architecture changes, ADR required
- **Strategic**: roadmap or project direction shifts, ADR + roadmap update

## ADR Workflow

- Copy a previous ADR as template
- Include: context, decision, consequences
- Set status: `Proposed`, `Accepted`, `Superseded`
- Reference related issues/PRs (when available)

## Review Expectations

For significant decisions, seek at least one maintainer review and explicitly confirm:

- security implications
- backward compatibility impact
- migration guidance (if needed)

## Source of Truth

- `docs/decision-log.md`: summary timeline
- `docs/adr/*.md`: detailed immutable decisions
- `docs/roadmap.md`: prioritized execution path
- `CHANGELOG.md`: user-visible release history
