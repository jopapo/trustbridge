# ADR-0008: Auto-Bootstrap CA Tooling in Targets

- Status: Accepted
- Date: 2026-08-07

## Context

Many containers/images/VMs lack `update-ca-certificates` or equivalent tooling. Manual preconditioning is impractical for broad developer workflows.

## Decision

When CA update tooling is missing, attempt to install `ca-certificates` using available package manager with root privileges.

Supported package managers include:

- `apt-get`, `apk`, `dnf`, `yum`, `microdnf`, `zypper`, `pacman`

Applies to:

- runtime VM patching
- running container patching
- image patching in temporary containers

## Consequences

### Positive

- higher compatibility across diverse base images
- reduced manual intervention
- better out-of-the-box behavior

### Trade-offs

- package installation may fail in locked-down or offline images
- increased mutation scope and runtime execution cost
