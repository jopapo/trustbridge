# ADR-0001: Rust + Provider Architecture

- Status: Accepted
- Date: 2026-08-07

## Context

TrustBridge needs to be reliable, extensible, and safe for runtime trust operations.

## Decision

Use Rust for implementation and provider traits for source/target integrations.

## Consequences

### Positive

- safer memory/model guarantees
- strong CLI ecosystem
- cleaner long-term extensibility

### Trade-offs

- steeper contributor learning curve vs scripting languages
- slower early prototyping for some contributors
