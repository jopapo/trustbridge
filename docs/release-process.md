# Release Process

This document defines the release checklist and commands for TrustBridge.

## Preconditions

- `main` is passing CI (`fmt`, `build`, `test`).
- Release notes for user-visible changes are prepared in `CHANGELOG.md`.

## Checklist

- [ ] Confirm latest `main` is green in GitHub Actions.
- [ ] Update `CHANGELOG.md`:
  - [ ] Move items from `## [Unreleased]` into a new version section.
  - [ ] Add release date (`YYYY-MM-DD`).
- [ ] Commit and push changelog update.
- [ ] Create annotated git tag `vX.Y.Z`.
- [ ] Push tag to origin.
- [ ] Wait for `release.yml` workflow completion.
- [ ] Verify GitHub Release has all artifacts:
  - Linux binary
  - macOS binary
  - Windows binary
- [ ] Recreate/update `## [Unreleased]` section for next iteration.

## Commands

```bash
git checkout main
git pull

# edit CHANGELOG.md

git add CHANGELOG.md
git commit -m "docs(changelog): release vX.Y.Z"
git push

git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

## Notes

- Release workflow triggers on pushed tags matching `v*`.
- Release assets are generated from CI builds across supported OS targets.
- If a release build fails, fix on `main`, then create a new tag (for example `vX.Y.Z+1`).
