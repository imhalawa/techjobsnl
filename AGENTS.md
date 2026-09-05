# TechJobsNL engineering

## Required context

- **Migration:** While [`GOAL.md`](GOAL.md) exists, read it and [`docs/MIGRATION_POLICY.md`](docs/MIGRATION_POLICY.md)
  before planning, implementing, reviewing, or orchestrating migration work.
- **.NET:** Before writing or reviewing .NET code, read [`docs/DOTNET_CONVENTIONS.md`](docs/DOTNET_CONVENTIONS.md).
- **Architecture:** Before changing project references, module ownership, interfaces, adapters, persistence, composition, or
  platform integration, read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and
  [`docs/architecture.mermaid`](docs/architecture.mermaid).
- **Domain language:** Before naming product concepts, read [`CONTEXT.md`](CONTEXT.md). Add a term only when its owning
  release becomes Current; do not preload future-release language.
- **Private planning:** Before reading or mutating GitHub planning records, read
  [`docs/PLANNING_PRIVACY.md`](docs/PLANNING_PRIVACY.md). Work items remain private Project draft items for their entire
  lifecycle.
- **Release:** Before activating, shipping, or attributing defects to a release, read
  [`docs/RELEASE_PROCESS.md`](docs/RELEASE_PROCESS.md) and [`RELEASES.md`](RELEASES.md).

## Execution

- Treat `era` as the migration default branch and target. Treat `main` as the historical Rust line until migration close.
- Preserve `archive/rust/` as the reference implementation. Modify it only through a task explicitly scoped to archive
  integrity.
- Implement only one assigned Ready task from the Current release. Confirm its dependencies are integrated into `era`.
- Apply the task's acceptance criteria, exact Rust references, expected change surface, and validation commands without
  expanding its scope.
- Report dependency conflicts, reference contradictions, unsafe migrations, and newly discovered work instead of silently
  resolving them beyond the task.
- Finish with acceptance evidence, validation results, changed files, compatibility impact, risks, follow-up proposals, and
  the conventional commit hash.
