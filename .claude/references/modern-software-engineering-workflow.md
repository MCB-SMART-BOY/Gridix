# Modern Software Engineering Workflow

## Purpose

This workflow is language-agnostic and applies to most software projects. Use it for feature work, bug fixes, performance optimization, refactoring, testing, CI hardening, and release preparation.

Project-specific rules should be treated as an overlay. For Gridix, use:

- `~/.codex/references/project-refactor-execution-plan.md`
- `~/.codex/references/workbench-ui-refactor-spec.md`
- `~/.codex/rules/*.md`

## Operating Principles

1. Preserve behavior before changing structure.
2. Make small, reviewable, reversible changes.
3. Separate refactoring from behavior changes unless the behavior change requires the refactor.
4. Use tests to capture current behavior before high-risk changes.
5. Prefer explicit boundaries over clever abstractions.
6. Optimize only after measuring or identifying a concrete bottleneck.
7. Keep the main branch deployable.
8. Treat documentation, configuration, migrations, tests, and observability as part of the feature.

## Work Types

Classify the task before changing code:

| Work type | Primary goal | Main risk | Required guardrail |
|---|---|---|---|
| Feature | New user-visible behavior | scope creep | acceptance criteria |
| Bug fix | Correct wrong behavior | incomplete reproduction | regression test |
| Refactor | Improve structure without behavior change | accidental behavior change | characterization tests |
| Optimization | Improve speed/memory/build time | premature tuning | baseline measurement |
| Test hardening | Improve confidence | brittle tests | stable test boundaries |
| Migration | Change data/config/API shape | compatibility break | migration and rollback |
| Release | Ship a known state | stale docs/artifacts | release checklist |

## Stage 0: Intake And Classification

Inputs:
- User request, issue, incident, failing test, performance target, or refactor goal.

Actions:
1. Restate the intended outcome.
2. Classify the work type.
3. Identify affected users, code paths, data, and APIs.
4. Decide whether the change is safe to do directly or needs a written plan.
5. Check for existing local changes before editing.

Exit criteria:
- Scope is bounded.
- Success criteria are explicit.
- Risk level is known.

## Stage 1: Discovery

Goal: understand the current system before changing it.

Actions:
1. Read project docs and local rules.
2. Locate entry points, data flow, and ownership boundaries.
3. Find existing patterns and tests.
4. Identify invariants that must not break.
5. For bugs, reproduce or explain why reproduction is not possible.
6. For optimization, capture a baseline measurement.

Recommended commands:

```bash
git status --short
find . -maxdepth 3 -type f | sort | sed -n '1,120p'
```

Prefer project-native search tools:

```bash
rg "symbol_or_error_text" .
rg --files
```

Fallback if `rg` is unavailable:

```bash
grep -RIn "symbol_or_error_text" .
find . -type f
```

Exit criteria:
- You know where the change belongs.
- You know how to verify the change.
- You know which risks need tests.

## Stage 2: Design

Goal: choose a minimal, defensible approach.

Actions:
1. Write the target behavior.
2. Define non-goals.
3. Identify module/API/config/data impacts.
4. Decide the migration order.
5. Define the test plan before implementation.
6. Decide rollback strategy.

Design checks:
- Can this be done behind an adapter?
- Can the old and new paths coexist temporarily?
- Is there a smaller first slice?
- Are tests possible at a lower layer?
- Does this introduce a new dependency? If yes, justify it.

Exit criteria:
- Implementation steps are ordered.
- Verification is concrete.
- No high-risk ambiguity remains.

## Stage 3: Safety Net

Goal: add or identify tests that protect intended behavior.

Use by work type:

| Work type | Safety net |
|---|---|
| Feature | unit tests for logic, integration tests for flow |
| Bug fix | failing regression test first when practical |
| Refactor | characterization tests around existing behavior |
| Optimization | benchmark or repeatable measurement |
| Migration | old-format load test and new-format save test |
| UI | state/reducer tests plus minimal render tests |

Rules:
- Prefer deterministic tests over broad snapshots.
- Test stable behavior, not implementation details.
- Do not write a brittle test just to increase coverage.
- If a test cannot be added, document why and use manual verification.

Exit criteria:
- There is a way to detect the main failure mode.

## Stage 4: Implementation

Goal: change code in small, verifiable slices.

Process:
1. Make the smallest structural change.
2. Compile/check.
3. Add behavior.
4. Run targeted tests.
5. Repeat until complete.

Refactoring order:
1. Extract pure helper.
2. Move helper to new module.
3. Add adapter/facade.
4. Switch call sites gradually.
5. Remove old path only after tests pass.

Feature order:
1. Domain model/config.
2. Internal API.
3. State/reducer.
4. UI/transport.
5. Persistence/migration.
6. Docs and tests.

Exit criteria:
- Targeted tests pass.
- Code is locally coherent.
- No unrelated cleanup is mixed in.

## Stage 5: Review

Goal: find defects before the user or CI does.

Review checklist:
- Does this solve the stated problem?
- Are edge cases handled?
- Are errors explicit and actionable?
- Are data/config migrations backward compatible?
- Are old paths removed only when safe?
- Does the change preserve existing invariants?
- Are tests meaningful and not overfitted?
- Are docs/config examples updated?
- Is performance acceptable?
- Is the diff reviewable?

Refactor-specific checks:
- Behavior changes are absent or explicitly documented.
- Public APIs remain compatible unless planned.
- Adapters are temporary and tracked.
- Large moves are not mixed with logic rewrites.

Exit criteria:
- You can explain the diff in a few sentences.
- Risks are either tested or documented.

## Stage 6: Verification

Goal: run the right checks for the change size.

Use three levels:

### Fast loop

Run frequently while editing:

```bash
cargo check
cargo test -p <package> <focused_test>
```

Equivalent for other stacks:

```bash
npm test -- <focused_test>
pytest path/to/test.py -q
go test ./pkg/name
```

### Pre-commit gate

Run before finalizing local changes:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -p <package>
```

### Pre-merge gate

Run before push/merge/release:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

Adapt commands to the project. The principle is fixed: format, lint, test, build docs/artifacts where relevant.

Exit criteria:
- Required gates pass.
- Any skipped check is explicitly reported with reason.

## Stage 7: Delivery

Goal: ship a clear, traceable change.

Actions:
1. Summarize behavior and structure changes.
2. Mention tests run.
3. Mention known risks or skipped checks.
4. Update changelog/release notes for user-visible changes.
5. Keep commit/PR focused.

Commit message:

```text
type(scope): concise imperative summary

- What changed
- Why
- Verification
```

Common types:
- `feat`
- `fix`
- `refactor`
- `perf`
- `test`
- `docs`
- `chore`
- `build`
- `ci`

## Refactoring Method

Use this sequence for safe refactors:

1. Characterize current behavior.
2. Extract names for unclear concepts.
3. Split pure logic from side effects.
4. Introduce a seam or adapter.
5. Move code behind the seam.
6. Switch callers one group at a time.
7. Delete obsolete code.
8. Re-run full tests.

Stop if:
- You need to change behavior to complete a "pure refactor".
- Tests fail for unclear reasons.
- You need broad search/replace across unrelated structs.
- The diff becomes too large to review.

## Optimization Method

Use this sequence for performance work:

1. Define target metric: latency, throughput, memory, binary size, build time, CPU, IO.
2. Capture baseline with a repeatable command.
3. Identify hot path with profiling or instrumentation.
4. Make one change.
5. Re-measure.
6. Keep the change only if it improves the target without unacceptable tradeoffs.

Do not:
- Optimize code that is not on the hot path.
- Replace clear code with complex code without measurement.
- Compare one noisy run before/after.

Recommended metrics:
- p50/p95/p99 latency for services
- memory peak and allocations
- query count and query time
- frame time for UI
- compile time for developer workflow
- binary/package size for distribution

## Testing Strategy

Use the test pyramid, but adapt it to the system:

```text
Many: pure unit tests
Some: integration tests
Few: end-to-end tests
Targeted: property/fuzz/benchmark tests where useful
```

Test types:

| Type | Use for | Avoid |
|---|---|---|
| Unit | pure logic and edge cases | testing private implementation details excessively |
| Integration | module boundaries and IO | testing every small branch |
| Contract | API compatibility | duplicating implementation |
| Regression | bug reproduction | vague assertions |
| Property | parsers, serializers, transforms | overly broad generators |
| Fuzz | untrusted input parsers | business logic with unclear oracle |
| Snapshot | stable rendered output/config | dynamic or noisy output |
| Benchmark | optimization claims | correctness checks |
| E2E | critical user journeys | covering all combinations |

## Modern Engineering Practices

Default recommendations:
- Use CI gates for format, lint, tests, security scanning, and docs where practical.
- Use dependency update automation with review, not blind auto-merge.
- Use feature flags for risky paths.
- Use structured logs/tracing for async and distributed systems.
- Use typed configuration and validated environment parsing.
- Use migration tests for schemas/config files.
- Use reproducible dev environments when possible.
- Keep generated files clearly separated.
- Use lockfiles for applications.
- Pin toolchain versions for reproducibility.

## AI Agent Rules

When using an AI coding agent:

1. Build context from the repo first.
2. Prefer small patches.
3. Run commands instead of guessing.
4. Do not overwrite unrelated user changes.
5. Report exact checks run.
6. Make unverified assumptions explicit.
7. Avoid broad destructive commands.
8. Keep final summaries short and actionable.

## Universal Definition Of Done

A change is done when:

- The stated problem is solved.
- Relevant tests pass.
- Formatting/linting pass or skipped checks are justified.
- Documentation/config examples are updated if behavior changed.
- Migration/rollback is considered if persisted data or APIs changed.
- No unrelated changes are included.
- Remaining risks are known.
