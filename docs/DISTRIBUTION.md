# Distribution Guide | 多平台分发指南

This guide describes how to sync a released Gridix version to AUR, Homebrew, and nixpkgs.
本文说明 Gridix 发布后如何同步到 AUR、Homebrew 与 nixpkgs。

This guide is step 4 of the major-phase release flow:
这份文档是“大阶段发布流程”的第 4 步：

1. bump version
2. commit + push branch
3. publish GitHub release
4. sync downstream package channels

Do not update downstream package definitions before the GitHub release and checksums are final.
在 GitHub release 和校验和最终确定之前，不要提前更新下游包定义。

## Current Status Snapshot | 当前状态快照

`v6.1.0` status:

- GitHub release: published
- AUR: synced for `v6.1.0`
- Homebrew: synced for `v6.1.0`
- nixpkgs: branch and PR updated to `v6.1.0`; upstream review/merge still pending

This means the release version is already published on GitHub, AUR/Homebrew are synced, and the remaining downstream work is the external review/merge cycle for nixpkgs.
这表示当前版本已经在 GitHub 上发布完成，AUR 与 Homebrew 已同步，剩余下游动作只剩 nixpkgs 的上游审查与合并流程。

## 1. Prerequisites | 前置条件

- GitHub release for target version already exists (`vX.Y.Z`).
  目标版本的 GitHub Release 已发布（`vX.Y.Z`）。
- Release assets are complete:
  发布制品完整：
  - `gridix-linux-x86_64.tar.gz`
  - `gridix-macos-arm64.tar.gz`
  - `gridix-windows-x86_64.zip`
  - `gridix.AppImage`
  - `SHA256SUMS.txt`
- Local workspace includes:
  本地工作区包含：
  - `_work_aur_gridix`
  - `_work_aur_gridix_bin`
  - `_work_aur_gridix_appimage`
  - `_work_homebrew_gridix`
  - `_work_nixpkgs`

## 2. Get Checksums | 获取校验和

```bash
VERSION=6.1.0
gh release download "v${VERSION}" -p "SHA256SUMS.txt" -D /tmp/gridix-release
cat /tmp/gridix-release/SHA256SUMS.txt
```

Do not update packaged formula hashes before the `v${VERSION}` artifacts exist.
在 `v${VERSION}` 制品生成前，不要提前改包管理器模板中的哈希。

Also fetch source tarball hash:
同时获取源码包哈希：
```bash
curl -L "https://github.com/MCB-SMART-BOY/Gridix/archive/refs/tags/v${VERSION}.tar.gz" -o /tmp/gridix-release/Gridix-v${VERSION}-source.tar.gz
sha256sum /tmp/gridix-release/Gridix-v${VERSION}-source.tar.gz
```

## 3. Update AUR | 更新 AUR

### 3.1 `gridix` (source package)
```bash
cd _work_aur_gridix
# update PKGBUILD: pkgver + source sha256
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "gridix: update to vX.Y.Z"
git push origin master
```

### 3.2 `gridix-bin` (binary package)
```bash
cd _work_aur_gridix_bin
# update PKGBUILD: pkgver + linux tar + desktop + icon + license sha256
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "gridix-bin: update to vX.Y.Z"
git push origin master
```

### 3.3 `gridix-appimage` (optional but recommended)
```bash
cd _work_aur_gridix_appimage
# update PKGBUILD: pkgver + AppImage + LICENSE sha256
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "gridix-appimage: update to vX.Y.Z"
git push origin master
```

## 4. Update Homebrew | 更新 Homebrew

```bash
cd _work_homebrew_gridix
# update Formula/gridix.rb: version + sha256 (linux/macos)
git add Formula/gridix.rb
git commit -m "gridix vX.Y.Z"
git push origin master
```

## 5. Update nixpkgs | 更新 nixpkgs

1. If Gridix already exists in nixpkgs, update `pkgs/by-name/gr/gridix/package.nix`:
   如果 nixpkgs 已有 Gridix，则更新 `pkgs/by-name/gr/gridix/package.nix`：
   - `version`
   - `src.hash`
   - `cargoHash`
2. If Gridix does **not** yet exist on the target base branch, create a clean branch from `nixos/master` and add both:
   如果目标基线分支上**还没有** Gridix，则从 `nixos/master` 切一条干净分支，并同时新增：
   - `pkgs/by-name/gr/gridix/package.nix`
   - `maintainers/maintainer-list.nix` entry for the package maintainer
3. Build-check locally:
   本地构建检查：
   ```bash
   cd _work_nixpkgs
   nix-instantiate -A gridix
   nix-build -A gridix
   ```
   If `cargoHash` is wrong, trust the hash reported by Nix and rerun `nix-build`.
   如果 `cargoHash` 不对，以 Nix 报出的哈希为准更新后再重跑。
4. Push branch to fork and update/create PR.
   推送分支到 fork 并更新或新建 PR。

## 6. Verification Checklist | 校验清单

- AUR `PKGBUILD` and `.SRCINFO` are consistent.
  AUR 的 `PKGBUILD` 与 `.SRCINFO` 一致。
- Homebrew formula URL and hash match release assets.
  Homebrew Formula 的 URL 与哈希与 release 产物一致。
- nixpkgs build succeeds with updated hashes.
  nixpkgs 使用新哈希后构建通过。
- Root README install section links remain valid.
  根 README 的安装链接保持有效。
