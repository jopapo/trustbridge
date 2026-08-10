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

## GitHub Repository Settings Required

Release Please needs repository-level GitHub Actions permissions to create PRs.

In repository settings:

1. `Settings` → `Actions` → `General`
2. Under `Workflow permissions`, select `Read and write permissions`
3. Enable `Allow GitHub Actions to create and approve pull requests`

Without this, Release Please can push branches but fails when opening the release PR.

## First Release with Release Please

1. Merge this Release Please setup to `main`.
2. Wait for `Release Please` workflow to open the first release PR.
3. Review the release PR contents (version bump + changelog updates).
4. Merge the release PR.
5. Confirm tag creation (`vX.Y.Z`) in repository tags.
6. Confirm `Release` workflow ran for that tag and published artifacts.
7. Validate GitHub Release page assets and notes.
