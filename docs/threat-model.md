# Threat Model (Initial)

## Assets

- Integrity of runtime truststore
- Reliability of developer network/TLS workflows
- Traceability of certificate changes

## Risks

1. Importing unintended/untrusted certificates
2. Removing required certificates from target truststore
3. Partial apply leaving runtime in broken state
4. Operator confusion due to hidden side effects

## Mitigations

- explicit `plan` before `apply`
- fingerprint-based diff and tracking
- deterministic bundle generation
- rollback strategy for failed apply
- minimal privileges and no private key handling

## Security Baseline

- sync only PEM public certificates
- never read/export private keys
- default to transparent logs and explicit operations
