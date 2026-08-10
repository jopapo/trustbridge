# Release Process

This document defines the release checklist and commands for TrustBridge.

## Preconditions

- `main` is passing CI (`fmt`, `build`, `test`).
- Commits and PR titles follow conventional commit semantics (`feat`, `fix`, etc.).
- PR title semantic check workflow is green.

## Automated Flow (Release Please)

- [ ] Merge changes into `main` using conventional commits.
- [ ] Wait for `Release Please` workflow to open/update a release PR.
- [ ] Review release PR (version bump + changelog).
- [ ] Merge release PR.
- [ ] Confirm generated tag `vX.Y.Z` was created.
- [ ] Confirm `release.yml` ran from the tag.
- [ ] Verify GitHub Release contains Linux/macOS/Windows artifacts.

## Option B (Recommended): Squash + Semantic PR Titles

Use this mode to keep release automation predictable and avoid Release Please skipping changes due to non-semantic merge commits.

Repository settings:

1. Enable `Allow squash merging`
2. Disable `Allow merge commits`
3. Optional: disable `Allow rebase merging` for stricter history
4. Protect `main` with required checks:
   - `CI / build-and-test`
   - `PR Title SemVer Check / semantic-pr-title`

Why this works:

- Every merged PR becomes one commit.
- The squash commit title comes from PR title.
- With semantic PR titles (`feat:`, `fix:`), Release Please can infer version bumps reliably.

## Manual Override (Optional)

```bash
git checkout main
git pull
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

## Notes

- `Release Please` workflow runs on pushes to `main` and creates release PRs automatically.
- Existing `release.yml` still publishes binaries on pushed tags (`v*`).
- Pre-releases can still be created manually with tags like `v0.2.0-alpha.1` when needed.
- Existing release PRs should be merged with the same squash policy for consistent history.

## Force Version Bump in a Specific PR

If a specific PR (including `chore:`) must force an exact release version, add this line
to the squash commit body when merging:

```text
Release-As: X.Y.Z
```

Example:

```text
Release-As: 0.2.0
```

Tips:

- Keep the PR title semantic (for example `chore: cleanup release docs`).
- Use squash merge so the final commit body includes `Release-As`.
- `Release-As` overrides normal `feat/fix` bump inference for that commit.

## GitHub Repository Settings Required

Release Please needs repository-level GitHub Actions permissions to create PRs.

In repository settings:

1. `Settings` → `Actions` → `General`
2. Under `Workflow permissions`, select `Read and write permissions`
3. Enable `Allow GitHub Actions to create and approve pull requests`

Without this, Release Please can push branches but fails when opening the release PR.

## Token Setup for Automatic Downstream Release Pipeline

To ensure the binary release workflow triggers automatically after Release Please creates tags/releases,
configure a dedicated PAT secret:

1. Create a fine-grained PAT (or classic PAT) for the repository owner/bot
2. Grant repository permissions for:
   - `Contents: Read and write`
   - `Pull requests: Read and write`
   - `Workflows: Read and write` (classic PAT uses `workflow` scope)
3. Add secret `RELEASE_PLEASE_TOKEN` in repository secrets
4. `release-please.yml` will prefer `RELEASE_PLEASE_TOKEN` and fallback to `GITHUB_TOKEN`

Why: events created by `GITHUB_TOKEN` often do not trigger other workflows; a PAT avoids that suppression.

## First Release with Release Please

1. Merge this Release Please setup to `main`.
2. Wait for `Release Please` workflow to open the first release PR.
3. Review the release PR contents (version bump + changelog updates).
4. Merge the release PR.
5. Confirm tag creation (`vX.Y.Z`) in repository tags.
6. Confirm `Release` workflow ran for that tag and published artifacts.
7. Validate GitHub Release page assets and notes.

## Quick Troubleshooting

- If Release Please says `No user facing commits found`, check whether merged commits were non-semantic merge commits.
- If no release PR appears, confirm the workflow has `Read and write` permissions and PR creation permission enabled.
- If binaries are missing in a tag release, check `Release` workflow artifacts and the `gh release upload` step logs.
