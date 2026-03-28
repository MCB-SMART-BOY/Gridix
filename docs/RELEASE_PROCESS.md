# Release Process | 发布流程

## 1. Trigger Model | 触发方式

Release workflow file: `.github/workflows/release.yml`  
发布工作流文件：`.github/workflows/release.yml`

Trigger:
- Push tag matching `v*`
  推送符合 `v*` 的 tag

Example:
```bash
git tag v3.3.0
git push origin v3.3.0
```

## 2. Build Matrix | 构建矩阵

Current release builds:
- Linux `x86_64-unknown-linux-gnu` -> `gridix-linux-x86_64.tar.gz`
- Windows `x86_64-pc-windows-msvc` -> `gridix-windows-x86_64.zip`
- macOS `aarch64-apple-darwin` -> `gridix-macos-arm64.tar.gz`
- AppImage job -> `gridix.AppImage`

Workflow also generates:
- `SHA256SUMS.txt`

## 3. Pre-Release Checklist | 发布前检查

1. Version in `Cargo.toml` is bumped.
   `Cargo.toml` 版本号已更新。
2. User-visible changes are documented in `docs/CHANGELOG.md`.
   `docs/CHANGELOG.md` 已记录用户可见变更。
3. Core docs are up to date (`README`, getting-started, keybindings, troubleshooting).
   核心文档（README/上手/键位/排障）已同步。
4. Local checks pass:
   本地检查通过：
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   python scripts/check_doc_links.py
   ```

## 4. Post-Release Verification | 发布后校验

1. Verify all assets exist on GitHub release page.
   确认 GitHub Release 制品齐全。
2. Verify `SHA256SUMS.txt` is generated and downloadable.
   确认 `SHA256SUMS.txt` 生成并可下载。
3. Smoke-test one install path:
   至少验证一条安装路径：
   - AUR or Homebrew or direct artifact
   - AUR / Homebrew / 直接下载任一方式
4. Sync downstream channels if needed (AUR/Homebrew/nixpkgs).
   按需同步下游通道（AUR/Homebrew/nixpkgs）。

## 5. Distribution Sync | 分发同步

After `vX.Y.Z` release is published, sync package channels in this order:
`vX.Y.Z` 发布后，按以下顺序同步包管理器渠道：

1. AUR: `gridix` -> `gridix-bin` -> `gridix-appimage`
2. Homebrew tap
3. nixpkgs PR branch

Reference doc:
- [DISTRIBUTION.md](DISTRIBUTION.md)

Useful commands:
```bash
# wait release workflow
gh run list --workflow release.yml --limit 5
gh run watch <run-id>

# download checksums
gh release download vX.Y.Z -p SHA256SUMS.txt -D /tmp/gridix-release
```

## 6. Common Failure Causes | 常见失败原因

- Tag format incorrect (missing leading `v`).
  tag 格式错误（缺少 `v` 前缀）。
- Platform dependency issue on build runner.
  构建 runner 缺少平台依赖。
- Asset naming mismatch with downstream formulas/specs.
  制品命名与下游公式/规范不一致。
- Changelog/docs not synchronized with release content.
  changelog/文档未和发布内容同步。

## 7. Rollback Strategy | 回滚策略

If severe issue is found:
1. Publish hotfix version with incremented tag.
   直接发布递增版本热修复。
2. Mark problematic release clearly in release notes.
   在发布说明中明确标注问题版本。
3. Update docs/changelog with corrective note.
   文档与变更日志补充修正说明。
