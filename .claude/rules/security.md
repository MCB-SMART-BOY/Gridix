---
paths:
  - src/data/config.rs
  - src/data/ssh_tunnel.rs
  - src/core/config.rs
---

# Gridix Security Rules

## DO

- Store passwords in OS keyring via `password_ref` UUID, NOT in config.toml
- Mark sensitive fields `#[serde(skip_serializing)]`
- Validate SSL certificates in Required mode (use system CA, not `danger_accept_invalid_certs`)
- Use `unwrap_or_else(|e| e.into_inner())` for Mutex poison recovery
- Log SSH host key SHA-256 fingerprints in `known_hosts` error messages
- Keep `pub(crate) mod app` — never make app module fully public

## DON'T

- Never write plaintext passwords to config files
- Never set `danger_accept_invalid_certs(true)` in SSL Required/Require modes
- Never use `danger_skip_domain_validation(true)` without explicit user opt-in
- Never use `std::thread::spawn` for SSH tunnel stop — use `Handle::spawn()`
- Never expose internal state types through `pub mod` — use `pub(crate)`

## VERIFY

```bash
# Check for plaintext password exposure
grep -r "password" src/core/config.rs | grep -v "password_ref\|skip_serializing"

# Check SSL certificate validation
grep -r "danger_accept_invalid_certs" src/data/pool.rs

# Check for thread::spawn usage (should only be in test code)
grep -r "std::thread::spawn" src/ --include='*.rs' | grep -v "#\[cfg(test)\]"

# Dependency policy and filesystem security scans
cargo deny check
gitleaks detect --no-git --source . --config .gitleaks.toml
trivy fs --scanners vuln,secret --skip-dirs target --skip-dirs .direnv --exit-code 1 .
```

`.gitleaks.toml` must keep `[extend] useDefault = true`; without it the file defines no
rules and every scan reports success regardless of content.
