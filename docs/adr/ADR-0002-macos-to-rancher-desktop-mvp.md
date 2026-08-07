# ADR-0002: MVP Scope macOS -> Rancher Desktop

- Status: Accepted
- Date: 2026-08-07

## Context

Initial user problem is corporate TLS trust mismatch on macOS with Rancher Desktop runtime.

## Decision

Ship first with:

- source: `macos-keychain`
- target: `rancher-desktop`

## Consequences

### Positive

- immediate value to initial user group
- reduced complexity and faster iteration cycle

### Trade-offs

- not immediately useful for Docker Desktop/Windows/Linux users
- target-specific implementation effort next
