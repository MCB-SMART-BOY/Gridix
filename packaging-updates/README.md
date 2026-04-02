# Gridix Packaging & Distribution Playbook | 分发更新手册

This folder stores release-time templates and check files.  
本目录存放发布时需要同步的包管理器模板与校验文件。

## 1. Files In This Folder | 目录内容
- `AUR-PKGBUILD`: source-build package template (`gridix`).
  AUR 源码包模板（`gridix`）。
- `AUR-PKGBUILD-bin`: binary package template (`gridix-bin`).
  AUR 二进制包模板（`gridix-bin`）。
- `AUR-PKGBUILD-appimage`: AppImage package template (`gridix-appimage`).
  AUR AppImage 包模板（`gridix-appimage`）。
- `homebrew-gridix.rb`: Homebrew formula template.
  Homebrew Formula 模板。
- `nixpkgs-package.nix`: nixpkgs package template.
  nixpkgs 包定义模板。
- `SHA256SUMS.txt`: release artifact checksums snapshot.
  当前发布版本的 SHA256 校验清单。
- `nixpkgs-PR-guide.md`: detailed nixpkgs PR workflow.
  nixpkgs PR 详细流程文档。

## 2. Release Update Order | 发布更新顺序
1. Publish GitHub release tag and artifacts first.
   先发布 GitHub tag 和制品。
2. Refresh checksums in `SHA256SUMS.txt`.
   更新 `SHA256SUMS.txt` 校验和。
3. Update AUR (`gridix`, `gridix-bin`, optional `gridix-appimage`).
   更新 AUR（`gridix`、`gridix-bin`、可选 `gridix-appimage`）。
4. Update Homebrew tap.
   更新 Homebrew tap。
5. Update nixpkgs branch/PR.
   更新 nixpkgs 分支与 PR。
6. Verify install commands on at least one machine per platform family.
   至少在每类平台上验证一次安装命令。

## 3. AUR Update | AUR 更新

### `gridix` (source package)
```bash
git clone ssh://aur@aur.archlinux.org/gridix.git
cd gridix
cp /path/to/Gridix/packaging-updates/AUR-PKGBUILD PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "gridix: update to vX.Y.Z"
git push
```

### `gridix-bin` (prebuilt package)
```bash
git clone ssh://aur@aur.archlinux.org/gridix-bin.git
cd gridix-bin
cp /path/to/Gridix/packaging-updates/AUR-PKGBUILD-bin PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "gridix-bin: update to vX.Y.Z"
git push
```

### `gridix-appimage` (AppImage package)
```bash
git clone ssh://aur@aur.archlinux.org/gridix-appimage.git
cd gridix-appimage
cp /path/to/Gridix/packaging-updates/AUR-PKGBUILD-appimage PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "gridix-appimage: update to vX.Y.Z"
git push
```

## 4. Homebrew Tap Update | Homebrew 更新
```bash
git clone https://github.com/MCB-SMART-BOY/homebrew-gridix.git
cd homebrew-gridix
cp /path/to/Gridix/packaging-updates/homebrew-gridix.rb Formula/gridix.rb
git add Formula/gridix.rb
git commit -m "gridix vX.Y.Z"
git push
```

## 5. nixpkgs Update | nixpkgs 更新
See full guide: [nixpkgs-PR-guide.md](nixpkgs-PR-guide.md)  
详细流程见：[nixpkgs-PR-guide.md](nixpkgs-PR-guide.md)

Minimal flow / 最小流程：
```bash
git clone https://github.com/YOUR_USERNAME/nixpkgs.git
cd nixpkgs
git remote add upstream https://github.com/NixOS/nixpkgs.git
git fetch upstream
git checkout -b gridix-X.Y.Z upstream/master
# update pkgs/by-name/gr/gridix/package.nix
nix-build -A gridix
git add pkgs/by-name/gr/gridix/package.nix
git commit -m "gridix: X.Y.Z -> X.Y.Z+1"
git push origin gridix-X.Y.Z
```

## 6. Verification Checklist | 发布校验清单
- GitHub release artifacts are complete and downloadable.
  GitHub Release 制品完整且可下载。
- `SHA256SUMS.txt` matches release artifacts.
  `SHA256SUMS.txt` 与 Release 制品一致。
- AUR package builds locally (`makepkg -si`).
  AUR 包可本地构建通过（`makepkg -si`）。
- Homebrew formula installs and launches.
  Homebrew Formula 可安装并启动。
- nixpkgs package builds (`nix-build -A gridix` or `nix build .#gridix`).
  nixpkgs 包可构建。
- README install section points to current channels.
  README 安装章节链接与通道保持最新。

## 7. Common Failures | 常见失败原因
- Version tag mismatch (`vX.Y.Z` vs `X.Y.Z`).
  版本字符串不一致（`vX.Y.Z` 与 `X.Y.Z`）。
- Wrong checksum after artifact replacement.
  替换制品后未更新 SHA256。
- nixpkgs `cargoHash` not refreshed.
  nixpkgs 的 `cargoHash` 未更新。
- AUR `.SRCINFO` not regenerated.
  AUR 未重新生成 `.SRCINFO`。

## 8. Maintenance Rule | 维护规则
- Update this folder in the same PR/commit set as release changes.
  发布相关改动应与本目录模板同步更新。
- Keep this doc generic; put version-specific values in template files.
  本文档保持流程化，版本具体值放到模板文件中维护。
