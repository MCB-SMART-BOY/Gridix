# Environment Variables | 环境变量

## 1. Runtime Logging | 运行日志

- `RUST_LOG`
  - Used by `tracing_subscriber::EnvFilter::try_from_default_env()`.
  - 默认回退为：`gridix=info,warn`

Example:
```bash
RUST_LOG=gridix=debug cargo run
```

## 2. MySQL Integration Test Variables | MySQL 集成测试变量

Used by `tests/mysql_cancel_integration.rs`:
由 `tests/mysql_cancel_integration.rs` 使用：

- `GRIDIX_IT_MYSQL_HOST`
- `GRIDIX_IT_MYSQL_PORT`
- `GRIDIX_IT_MYSQL_USER`
- `GRIDIX_IT_MYSQL_PASSWORD`
- `GRIDIX_IT_MYSQL_DB`

Example:
```bash
GRIDIX_IT_MYSQL_HOST=127.0.0.1 \
GRIDIX_IT_MYSQL_PORT=3306 \
GRIDIX_IT_MYSQL_USER=gridix \
GRIDIX_IT_MYSQL_PASSWORD=gridix \
GRIDIX_IT_MYSQL_DB=gridix_test \
cargo test --test mysql_cancel_integration -- --ignored --nocapture
```

## 3. CI Context | CI 环境

`mysql-integration.yml` sets the same `GRIDIX_IT_MYSQL_*` variables in workflow env.  
`mysql-integration.yml` 在 CI 中注入同名变量。

## 4. Notes | 说明

- Integration tests are ignored by default without required vars.
  集成测试默认被 `#[ignore]`，无变量时会跳过。
- Do not place real credentials in shell history for shared machines.
  在共享环境避免将真实凭据留在命令历史中。
