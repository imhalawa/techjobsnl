# Rust-to-.NET Migration Policy

This policy governs migration work while the root `GOAL.md` exists. It keeps the rewrite reversible, makes the archived
Rust product the behavioral reference, and constrains every change to an agreed issue.

## Branch policy

- Treat `era` as the migration default branch.
- Create every issue branch or worktree from the current `era` head.
- Target migration pull requests to `era`.
- Treat `main` as the historical Rust line during the migration.
- Merge an issue's declared dependencies into `era` before starting that issue.
- Rebase or recreate stale issue work on `era` before review rather than introducing compatibility shims for obsolete
  intermediate code.

## Reference implementation

- Treat `archive/rust/` as the complete reference implementation.
- Preserve the archive unchanged. An archive correction requires an issue explicitly scoped to reference integrity.
- Inspect the owning Rust implementation and its tests before specifying or porting behavior.
- Use these references by concern:
  - User workflows: `archive/rust/docs/USER_GUIDE.md` and `archive/rust/tests/ui_test.rs`.
  - Runtime ownership and safety: `archive/rust/docs/ARCHITECTURE.md`.
  - Configuration and compatibility: `archive/rust/docs/CONFIGURATION.md`, `archive/rust/config.toml`, and
    `archive/rust/tests/config_test.rs`.
  - Persistence and lifecycle: `archive/rust/src/storage/`, `archive/rust/tests/storage_test.rs`, and
    `archive/rust/tests/scanner_test.rs`.
  - Source contracts: `archive/rust/SOURCES.md`, the matching adapter, its stored fixtures, and its offline and live tests.
  - Analytics and evidence: `archive/rust/src/analytics.rs`, `archive/rust/src/insights.rs`, and their tests.
  - Installation, CI, and releases: `archive/rust/scripts/`, `archive/rust/.github/workflows/`, and release tests.
- Resolve uncertain behavior from executable Rust tests and fixtures before prose. Record any genuine contradiction in the
  issue rather than choosing silently.

## Reversibility and compatibility

- Preserve observable product behavior unless an issue explicitly defines an approved change.
- Preserve existing TOML configuration and SQLite data. A format or schema change requires a tested forward migration and
  a documented recovery path.
- Keep stable `(company_id, source_id)` job identity and content/lifecycle meaning compatible.
- Keep startup local: loading the application does not contact external sources.
- Keep scans user-triggered and company outcomes isolated.
- Preserve Complete, Incomplete, and Failed semantics. Only Complete results may update jobs and close missing source IDs.
- Retain the last trusted jobs when a source result is incomplete or failed.
- Keep the archived application available until every `GOAL.md` completion condition has evidence and approval.

## Parity policy

- Port behavior through deterministic characterization or parity tests before replacing the Rust path.
- Reuse archived representative fixtures when their license and format permit direct reuse; otherwise create a minimal
  equivalent fixture that proves the same contract.
- Compare normalized outputs, persistence effects, state transitions, diagnostics, and user-visible workflow rather than
  merely matching type or function names.
- Treat a Rust defect discovered during migration as a separate behavior-change decision. First characterize current
  behavior; then fix it through an explicitly approved issue.
- Keep UI-specific behavior in the owning client while Core owns shared business rules and read models.
- Count a feature as migrated only when its issue acceptance criteria, deterministic tests, and declared platform checks
  pass.

## Issue boundaries

- Each implementation issue owns one independently reviewable behavior or foundation change.
- Each issue states scope, exclusions, dependencies, Rust references, acceptance criteria, validation commands, expected
  artifacts, and the sufficient agent/model capability.
- Keep commits small, conventional, buildable, and limited to the assigned issue.
- Add the smallest deterministic test that proves the requested behavior or parity.
- Create follow-up issues for newly discovered work instead of expanding the active issue.
- Update the local issue ledger only when the orchestrator assigns that responsibility.

## Issue execution

1. Read the assigned issue, its dependencies, acceptance criteria, and linked Rust references.
2. Confirm every dependency is merged into `era`.
3. Inspect the relevant archive code, tests, fixtures, and documentation.
4. Establish the required deterministic test or validation evidence.
5. Implement only the assigned scope using `docs/DOTNET_CONVENTIONS.md` and the target dependency direction.
6. Run focused checks followed by every validation command named by the issue.
7. Report acceptance-criterion evidence, remaining risks, and discovered follow-up work separately.

An issue is complete when every acceptance criterion has evidence, required deterministic tests pass, formatting and
architecture checks pass, no unrelated files changed, and the resulting commit is conventional and buildable.

## Policy retirement

The migration-closing issue deactivates this policy by deleting `GOAL.md` and removing the migration-policy pointer from
`AGENTS.md`. Preserve this file as historical migration rationale unless that closing issue explicitly replaces it with a
final migration record.
