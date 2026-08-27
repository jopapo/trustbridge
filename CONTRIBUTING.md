# Contributing

Thanks for considering a contribution to TrustBridge.

## Development setup

1. Install Rust stable toolchain.
2. Clone the repository.
3. Run:
   - `cargo fmt`
   - `cargo check`
   - `cargo test`

If dependency fetch fails in restricted networks, run setup in an environment with crates.io access or with a trusted internal mirror.

## Local developer run examples

When running from source during development, use:

```bash
cargo run -- scan
cargo run -- plan
cargo run -- apply --dry-run
cargo run -- apply
```

## Project structure

- `src/main.rs`: CLI entrypoint
- `src/cli.rs`: command and flags definitions
- `src/core/`: sync engine, plan, state, certificate model
- `src/providers/source/`: host trust source providers
- `src/providers/target/`: runtime target providers
- `src/providers/target/vm_backend.rs`: shared transport for VM-backed targets
- `src/commands/`: command handlers
- `docs/`: architecture, ADRs, security, release process

## Architecture flow (high-level)

1. Discover certificates from source provider
2. Normalize and fingerprint certificates
3. Diff source vs target trust state
4. Build sync plan (`to_add`, `to_remove`)
5. Apply changes (or dry-run)
6. Verify trust behavior

## Contribution guidelines

- Keep changes focused and minimal.
- Preserve provider contract boundaries.
- Prefer explicit errors over silent behavior.
- Update docs/ADRs when changing architecture or behavior.
- Update `CHANGELOG.md` for user-visible changes.

## Suggested workflow

1. Open an issue with problem and proposal.
2. Align scope with roadmap.
3. Submit PR with tests/docs updates.
4. Use semantic PR title (conventional commit style).
5. Merge via squash.

## Commit convention (recommended)

- `feat:` new user-facing capability
- `fix:` bug fix
- `docs:` documentation-only change
- `refactor:` structural improvement with no behavior change

For automated semantic versioning with Release Please:

- `feat:` triggers a MINOR release
- `fix:` triggers a PATCH release
- include `!` (for example `feat!:`) or a `BREAKING CHANGE:` footer to trigger a MAJOR release
- to force a specific version in a PR merge commit body, use `Release-As: X.Y.Z`

Example squash commit body line:

```text
Release-As: 0.2.0
```

## Merge strategy (version planning)

- Prefer squash merge for every PR.
- Avoid merge commits on `main`.
- Keep PR titles semantic because squash title becomes release-relevant commit.

## Changelog policy

TrustBridge maintains a human-curated changelog in `CHANGELOG.md`:

- Release Please manages release PRs, changelog updates, and version tagging from commit history.
- Keep commit messages/PR titles consistent with conventional commits.
- For manual changelog edits, preserve Keep a Changelog structure.

## Useful docs

- `docs/architecture.md`
- `docs/roadmap.md`
- `docs/decision-log.md`
- `docs/decision-process.md`
- `docs/release-process.md`
- `docs/threat-model.md`
- `docs/adr/`

## Cross-machine continuity

- Keep docs updated for environment-specific pitfalls (Windows native vs WSL vs Linux).
- Record recurring install/runtime decisions in ADRs and `docs/release-process.md`.
