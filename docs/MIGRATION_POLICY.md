# Rust-to-.NET Migration Policy

This policy governs migration work while the root `GOAL.md` exists. It keeps the rewrite reversible, makes the archived
Rust product the behavioral reference, and constrains every change to an agreed migration task.

## Branch policy

- Treat `era` as the migration default branch.
- Create every task branch or worktree from the current `era` head.
- Target migration pull requests to `era`.
- Treat `main` as the historical Rust line during the migration.
- Merge a task's declared dependencies into `era` before starting that task.
- Rebase or recreate stale task work on `era` before review rather than introducing compatibility shims for obsolete
  intermediate code.

## Reference implementation

- Treat `archive/rust/` as the complete reference implementation.
- Preserve the archive unchanged. An archive correction requires a task explicitly scoped to reference integrity.
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
  task rather than choosing silently.

## Reversibility and compatibility

- Preserve observable product behavior unless a task explicitly defines an approved change.
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
  behavior; then fix it through an explicitly approved task.
- Keep UI-specific behavior in the owning client while Core owns shared business rules and read models.
- Count a feature as migrated only when its task acceptance criteria, deterministic tests, and declared platform checks
  pass.

## Incremental delivery and tasks

Each release owns a small declared journey; full migration parity remains the final `GOAL.md` gate.
Core and presentation may be delivered together. Unsupported capabilities stay explicit: an unavailable
adapter never reports an empty Complete result and never removes retained company configuration or data.

`docs/ISSUE_TEMPLATE.md` owns concise task packets; `Workflow.md` owns execution; `docs/RELEASE_PROCESS.md`
owns shipping. Read task-named Rust files/cases, then expand only for a specific unresolved behavior.
Preserve all original scope, acceptance criteria, validation commands, artifacts, and review duties through
private traceability when splitting tasks. Completed work stays credited; Active work keeps its original contract.
Use the session-authorized git workflow from current `era`; task branches or isolated worktrees target `era`.

## Policy retirement

The migration-closing task deactivates this policy by deleting `GOAL.md` and removing the migration-policy pointer from
`AGENTS.md`. Preserve this file as historical migration rationale unless that closing task explicitly replaces it with a
final migration record.
