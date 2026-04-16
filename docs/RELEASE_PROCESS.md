# Release Process | 发布流程

## 0. Major-Phase Rule | 大阶段发版规则

When a large feature/recovery phase is finished, default to a major release cadence and execute the release in this exact order:
当一个大的功能/恢复阶段完成后，默认按“大版本发布”处理，并严格按这个顺序执行：

1. Bump the major version first.
   先更新大版本号。
2. Commit and push the phase-complete branch state.
   提交并推送阶段完成后的分支状态。
3. Publish the new GitHub release by pushing a `vX.Y.Z` tag.
   通过推送 `vX.Y.Z` tag 发布新的 GitHub Release。
4. Only after release assets and checksums are available, sync AUR, Homebrew, nixpkgs, and other downstream channels.
   只有在 release 制品和校验和都可用后，才同步 AUR、Homebrew、nixpkgs 等下游渠道。

Recommended command order:
建议命令顺序：
```bash
# 1. bump version/changelog/docs
git add Cargo.toml Cargo.lock docs/CHANGELOG.md docs/README.md docs/RELEASE_PROCESS.md docs/DISTRIBUTION.md
git commit -m "release: vX.0.0"
git push origin master

# 2. publish release
git tag vX.0.0
git push origin vX.0.0
```

If the scope is clearly not a breaking phase, you can still choose minor/patch release, but the default policy after a major phase is `major`.
如果这次范围明显不是破坏性阶段，也可以选择 minor/patch；但在“大阶段完成”之后，默认策略是升 `major`。

## 1. Trigger Model | 触发方式

Release workflow file: `.github/workflows/release.yml`  
发布工作流文件：`.github/workflows/release.yml`

Trigger:
- Push tag matching `v*`
  推送符合 `v*` 的 tag

Example:
```bash
git tag v6.1.0
git push origin v6.1.0
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
2. If this release closes a major feature/recovery phase, the version bump is a major bump by default.
   如果这次发布标志着一个大功能/恢复阶段结束，默认使用大版本号提升。
3. User-visible changes are documented in `docs/CHANGELOG.md`.
   `docs/CHANGELOG.md` 已记录用户可见变更。
4. Core docs are up to date (`README`, getting-started, keybindings, troubleshooting).
   核心文档（README/上手/键位/排障）已同步。
5. The branch commit is pushed before the release tag is pushed.
   先推送分支提交，再推送 release tag。
6. Local checks pass:
   本地检查通过：
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   python scripts/check_doc_links.py
   ```

## 3.5 Recovery-Phase Closure Checklist | recovery 阶段收口检查

Before treating a recovery stream as ready for release, confirm all of the following:
在把 recovery 主线视为可发版状态前，先确认以下条件全部成立：

1. `docs/recovery/10-master-recovery-plan.md` no longer lists any new unblocked active implementation workstream.
   `docs/recovery/10-master-recovery-plan.md` 已不再列出新的未阻塞 active implementation workstream。
2. `docs/recovery/12-bug-ledger-4.1.0.md` has either closed or downgraded the previously active bugs to `observation`.
   `docs/recovery/12-bug-ledger-4.1.0.md` 已将此前 active bug 关闭或降级为 `observation`。
3. Required live smokes recorded in `docs/TESTING.md` have at least one real run for the recovered high-risk flows.
   `docs/TESTING.md` 中要求的高风险恢复链路已至少做过一轮真实 live smoke。
4. Remaining risks are explicitly documented as `observation / testing gap / environment caveat`, not left as implied unfinished work.
   剩余风险已被明确写成 `observation / testing gap / environment caveat`，而不是含糊地留成未收口实现项。
5. The release decision itself is still explicit: phase-ready does not automatically mean `bump / tag / publish`.
   发版决定仍然必须显式确认：阶段 ready 不等于自动执行 `升版 / 打 tag / 发布`。

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

This distribution step is always after:
这个分发步骤始终在以下动作之后：

1. version bump
2. commit + push
3. GitHub release publication

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
