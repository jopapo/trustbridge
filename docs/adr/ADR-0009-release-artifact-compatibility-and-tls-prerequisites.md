# ADR-0009: Release Artifact Compatibility and TLS Prerequisites

- Status: Accepted
- Date: 2026-08-25

## Context

Users installing TrustBridge across Windows, WSL, Linux, and corporate environments reported recurring installation/runtime failures caused by:

- artifact/runtime mismatch (for example Windows users executing Linux artifacts in WSL unexpectedly)
- glibc baseline mismatch (`GLIBC_x.xx not found`) on Linux/WSL
- TLS interception without trusted corporate root CA (`self-signed certificate in certificate chain`)

These failures are operational and environment-driven, not product-logic defects, but they impact adoption and support load.

## Decision

1. Keep release distribution explicit by platform/runtime compatibility.
2. Publish both Linux variants:
   - `x86_64-unknown-linux-gnu`
   - `x86_64-unknown-linux-musl` (compatibility fallback)
3. Document artifact selection and TLS trust prerequisites in the main docs and release checklist.
4. Treat TLS trust bootstrap in host/WSL/CI environments as a prerequisite for network-dependent setup/build commands.

## Consequences

### Positive

- reduced installation failures in heterogeneous Linux/WSL environments
- clearer operator guidance for corporate TLS interception contexts
- better cross-machine reproducibility of onboarding and support

### Trade-offs

- additional release build time and artifact count
- documentation must be maintained as runtime/platform matrix evolves
