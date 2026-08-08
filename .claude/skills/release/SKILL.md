---
name: gridix-release
description: Execute the Gridix release checklist from bump to publish. Use only in the Gridix repository when asked to release, bump version, publish a release, or tag a version.
paths:
  - Cargo.toml
  - docs/CHANGELOG.md
---

# Release process

Pushing a `v*` tag triggers the CI build and release workflow; it does not demonstrate that a release was published. All commands run from repo root.

## 1. Pre-release validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
cargo run --bin check-doc-links
cargo audit
```

## 2. Release-acceptance evidence

- PostgreSQL and MySQL typed/cancellation Actions workflows must succeed for the candidate SHA. They run on PRs, `main`, and `v*` tags, preflight `GRIDIX_TEST_PG_URL` / `GRIDIX_TEST_MYSQL_URL`, and run serial integration tests with service containers.
- RA2 is still a manual SQLite GUI journey, not an automated driver check. Preserve initial-result, saved-result, and reopened-result screenshots under `/tmp/gridix-release-acceptance/<SHA>/`, plus non-empty `acceptance.csv`, `acceptance.json`, and `acceptance.sql`. The exports must show `after`, `"name":"after"`, and `'after'` with `NULL`, respectively.
- `gridix-driver` only supports launch, key, screenshot, quit, and help; it cannot complete dialog, text-entry, or export interactions. Do not mark RA2 accepted without the manual artifacts.
- Record the observed workflow run URLs and artifacts. Do not claim a release is published until the release result is observed.

## 3. Version bump

Edit `Cargo.toml`:
```toml
version = "X.Y.Z"
```

## 4. Changelog

Update `docs/CHANGELOG.md` — version header, date, categorized bullets (bilingual).

## 5. Related docs

Shortcut changes → update the `gridix-keybindings` skill. Config changes → update `CLAUDE.md`. Cross-check `docs/CHANGELOG.md`.

## 6. Commit + push (branch first, then tag)

```bash
git add Cargo.toml Cargo.lock docs/CHANGELOG.md
git commit -m "release: vX.Y.Z"
git push origin main
```

Wait for the candidate CI runs, including `ci.yml`, `postgresql-integration.yml`, and `mysql-integration.yml`.

## 7. Tag → trigger release

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Monitor the tag's `ci.yml` run and confirm the release job result before using `gh release view`.

## 8. Verify artifacts

```bash
gh release view vX.Y.Z
gh release download vX.Y.Z -p SHA256SUMS.txt -D /tmp/gridix-release
```

Expected: `gridix-linux-x86_64.tar.gz`, `gridix-windows-x86_64.zip`, `gridix-macos-arm64.tar.gz`, `gridix.AppImage`, `SHA256SUMS.txt`.

## 9. Distribution sync (post-release, manual)

Order: AUR (`gridix` → `gridix-bin` → `gridix-appimage`) → Homebrew → nixpkgs.

### Get checksums
```bash
VERSION=X.Y.Z
gh release download "v${VERSION}" -p SHA256SUMS.txt -D /tmp/gridix-release
curl -L "https://github.com/MCB-SMART-BOY/Gridix/archive/refs/tags/v${VERSION}.tar.gz" -o /tmp/gridix-release/source.tar.gz
sha256sum /tmp/gridix-release/source.tar.gz
```

### AUR (3 packages)
```bash
cd _work_aur_gridix       # source: update pkgver + source sha256 in PKGBUILD
cd _work_aur_gridix_bin    # binary: update pkgver + linux tar + desktop + icon + license sha256
cd _work_aur_gridix_appimage  # appimage: update pkgver + AppImage + LICENSE sha256
# Each: makepkg --printsrcinfo > .SRCINFO && git add PKGBUILD .SRCINFO && git commit -m "update to vX.Y.Z" && git push
```

### Homebrew
```bash
cd _work_homebrew_gridix
# Update Formula/gridix.rb: version + sha256 (linux/macos)
git add Formula/gridix.rb && git commit -m "gridix vX.Y.Z" && git push origin master
```

### nixpkgs
Update `pkgs/by-name/gr/gridix/package.nix`: `version`, `src.hash`, `cargoHash`. If new: also add `maintainers/maintainer-list.nix` entry.
```bash
cd _work_nixpkgs
nix-build -A gridix  # verify, trust Nix-reported hash if cargoHash wrong
# Push branch to fork, create/update PR
```

### Verify
- AUR PKGBUILD + .SRCINFO consistent
- Homebrew formula URL + hash match release assets
- nixpkgs build succeeds

## Rollback

If severe issue: publish hotfix with incremented patch, mark broken release, update changelog.
