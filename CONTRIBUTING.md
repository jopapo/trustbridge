# Contributing

Thanks for considering a contribution to TrustBridge.

## Development Setup

1. Install Rust stable toolchain.
2. Clone repository.
3. Run:
   - `cargo fmt`
   - `cargo check`
   - `cargo test`

## Contribution Guidelines

- keep changes focused and minimal
- preserve provider contract boundaries
- prefer explicit errors over silent behavior
- update docs/ADRs when changing architecture or behavior

## Suggested Workflow

1. Open an issue with problem and proposal.
2. Align scope with roadmap.
3. Submit PR with tests/docs updates.

## Commit Convention (recommended)

- `feat:` new user-facing capability
- `fix:` bug fix
- `docs:` documentation-only change
- `refactor:` structural improvement with no behavior change
