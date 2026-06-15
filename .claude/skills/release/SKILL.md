---
name: release
description: Execute the Gridix release checklist from bump to publish. Use when asked to release, bump version, publish a release, or tag a version.
paths:
  - Cargo.toml
  - docs/CHANGELOG.md
---

# Release process

Tag-triggered: push `v*` tag → CI builds + publishes all artifacts.
All commands from repo root.

## 1. Pre-release checks

```bash
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test && python scripts/check_doc_links.py
```

## 2. Version bump

Edit `Cargo.toml`:
```toml
version = "X.Y.Z"
```

## 3. Changelog

Update `docs/CHANGELOG.md` — version header, date, categorized bullets (bilingual).

## 4. Related docs

Shortcut changes → update `/keybindings` skill. Config changes → update `CLAUDE.md`. Cross-check `docs/CHANGELOG.md`.

## 5. Commit + push (branch first, then tag)

```bash
git add Cargo.toml Cargo.lock docs/CHANGELOG.md
git commit -m "release: vX.Y.Z"
git push origin master
```

Wait for CI: `gh run list --workflow build.yml --limit 3`

## 6. Tag → trigger release

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Monitor: `gh run list --workflow release.yml --limit 3`

## 7. Verify artifacts

```bash
gh release view vX.Y.Z
gh release download vX.Y.Z -p SHA256SUMS.txt -D /tmp/gridix-release
```

Expected: `gridix-linux-x86_64.tar.gz`, `gridix-windows-x86_64.zip`, `gridix-macos-arm64.tar.gz`, `gridix.AppImage`, `SHA256SUMS.txt`.

## 8. Distribution sync (post-release, manual)

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
