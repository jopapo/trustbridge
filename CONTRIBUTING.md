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
- update `CHANGELOG.md` for user-visible changes

## Suggested Workflow

1. Open an issue with problem and proposal.
2. Align scope with roadmap.
3. Submit PR with tests/docs updates.

## Commit Convention (recommended)

- `feat:` new user-facing capability
- `fix:` bug fix
- `docs:` documentation-only change
- `refactor:` structural improvement with no behavior change

For automated semantic versioning with Release Please:

- `feat:` triggers a MINOR release
- `fix:` triggers a PATCH release
- include `!` (for example `feat!:`) or a `BREAKING CHANGE:` footer to trigger a MAJOR release

## Changelog Policy

TrustBridge maintains a human-curated changelog in `CHANGELOG.md`:

- Release Please now manages release PRs, changelog updates, and version tagging from commit history.
- Keep commit messages/PR titles consistent with conventional commits.
- For manual changelog edits, preserve Keep a Changelog structure.
