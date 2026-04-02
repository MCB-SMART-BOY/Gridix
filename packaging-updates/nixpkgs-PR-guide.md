# nixpkgs PR Guide | 提交指南

This document is for updating or creating Gridix package changes in nixpkgs.  
本文用于在 nixpkgs 中更新或提交 Gridix 包。

## 1. Prepare Branch | 准备分支
```bash
git clone https://github.com/YOUR_USERNAME/nixpkgs.git
cd nixpkgs
git remote add upstream https://github.com/NixOS/nixpkgs.git
git fetch upstream
git checkout -b gridix-X.Y.Z upstream/master
```

If you already have an open PR branch, reuse it and `git rebase upstream/master`.
如果你已有在审 PR 分支，优先复用并 `git rebase upstream/master`。

## 2. Update Package File | 更新包文件
- Target path: `pkgs/by-name/gr/gridix/package.nix`
- Replace with template from:
  `/path/to/Gridix/packaging-updates/nixpkgs-package.nix`

Key fields to update / 关键字段：
- `version`
- `src.hash`
- `cargoHash`

## 3. Refresh `cargoHash` | 更新 cargoHash
```bash
nix-build -A gridix
# or
nix build .#gridix
```

If hash mismatch appears, copy the suggested hash from output and rebuild.
若出现哈希不匹配，复制输出提示的新 hash，更新后再次构建。

## 4. Commit & Push | 提交与推送
```bash
git add pkgs/by-name/gr/gridix/package.nix
git commit -m "gridix: OLD_VERSION -> NEW_VERSION"
git push origin gridix-X.Y.Z
```

## 5. Open/Update PR | 创建或更新 PR
- Base: `NixOS/nixpkgs:master`
- Title example:
  - New package: `gridix: init at X.Y.Z`
  - Version bump: `gridix: OLD_VERSION -> NEW_VERSION`

Suggested PR body / 建议 PR 描述：
```md
## Summary
- Update gridix to NEW_VERSION.

## Testing
- [x] `nix-build -A gridix` passes on x86_64-linux

## Notes
- Source: https://github.com/MCB-SMART-BOY/Gridix/releases/tag/vNEW_VERSION
```

## 6. Why PR Fails (Common) | PR 常见失败原因
- `cargoHash` mismatch.
  `cargoHash` 未更新或错误。
- `src.hash` mismatch.
  `src.hash` 与 tag 源码不匹配。
- package lint/style issues in nixpkgs CI.
  nixpkgs CI 的格式或规范检查失败。
- branch out-of-date with upstream.
  分支未同步最新 `upstream/master`。

## 7. Fast Fix Loop | 快速修复流程
1. `git fetch upstream && git rebase upstream/master`
2. Fix hash / style issue
3. `nix-build -A gridix` again
4. `git push --force-with-lease`

## 8. Maintainer Note | 维护说明
- Keep this guide process-oriented.
  本文档保持流程化说明。
- Put version-specific values in `nixpkgs-package.nix`.
  具体版本值写入 `nixpkgs-package.nix` 模板。
