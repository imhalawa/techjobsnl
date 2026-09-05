# TechJobsNL migration

## Mission

Replace TechJobsNL's Rust implementation with a C#/.NET implementation while preserving observable behavior, local data,
source-safety guarantees, and the terminal workflow. Add an Avalonia desktop client and keep Core portable for later
Android and iOS clients. The migration is complete only after verified feature parity and supported-platform builds.

## Required context

- **Convention:** Before writing or reviewing .NET code, read and apply [`docs/DOTNET_CONVENTIONS.md`](docs/DOTNET_CONVENTIONS.md).
- **Rust reference:** When porting behavior, schemas, source adapters, configuration, UI workflows, tests, or release behavior,
  inspect the corresponding material under [`archive/rust/`](archive/rust/). Treat it as the reference implementation.
- **Product behavior:** For user workflows read `archive/rust/docs/USER_GUIDE.md`; for runtime invariants read
  `archive/rust/docs/ARCHITECTURE.md`; for configuration read `archive/rust/docs/CONFIGURATION.md`; for source contracts
  read `archive/rust/SOURCES.md` and the matching fixture tests.
- **Migration work:** Once the migration plan and issue ledger exist, read the issue named by the orchestrator and only the
  plan sections it points to.

## Branch and preservation rules

- Treat `era` as the migration's default branch. Create issue branches or worktrees from `era` and target pull requests to
  `era`; `main` is the historical Rust line during the migration.
- Keep `archive/rust/` intact as the reversible baseline. Modify it only when an issue explicitly concerns archive
  integrity or reference correction.
- Preserve compatibility with the existing TOML configuration and SQLite data unless an issue defines and tests a
  migration.
- Keep each change bounded to one issue, its acceptance criteria, and the smallest supporting documentation or tests.

## Target projects and dependencies

```text
TechJobsNL.Tui  ----\
                     >---- TechJobsNL.Core
TechJobsNL.App  ----/
```

Production projects:

- `TechJobsNL.Core`: Domain, Application, Persistence, Integrations, and Configuration.
- `TechJobsNL.Tui`: terminal rendering, input, navigation, and user interaction.
- `TechJobsNL.App`: Avalonia views, immutable ViewModel state, desktop navigation, dialogs, and themes.

Test projects mirror each production project. Core has no terminal, Avalonia, desktop, or mobile dependency. TUI and App
do not depend on each other and do not duplicate business rules.

Within Core, Domain depends on nothing; Application depends on Domain; Persistence, Integrations, and Configuration
implement Application-facing contracts. Namespaces exactly mirror folders. Keep Core types free of `partial`
declarations and platform APIs.

## Technology decisions

- .NET 10 LTS and stable C# 14; dependency injection, configuration, logging, and hosting use Microsoft extensions.
- SQLite persistence through Dapper and explicit lowercase SQL. Repositories expose behavior rather than generic CRUD.
- Refit external contracts return `ApiResponse<T>`. Typed request/response models use reflection-based System.Text.Json;
  adapters own named Polly resilience pipelines and map transport results into domain outcomes.
- Project-owned command/query dispatch uses explicit registrations and separate `ExecuteAsync` and `QueryAsync` entry
  points. It applies logging, timing, validation, then handler execution without MediatR, reflection, or assembly scanning.
- Bounded channels carry ordered scan/UI progress. In-process events are non-persistence-critical until a separate event
  reliability decision is approved.
- Avalonia uses CommunityToolkit.Mvvm. UI-required partial types stay in the App project; code-behind contains view-only
  mechanics. ViewModels replace immutable state snapshots.

## Product invariants

- Startup reads local data and does not contact sources until the user requests a scan.
- Every source uses an official company or ATS endpoint and produces stable job identity and trusted official URLs.
- A source result is Complete, Incomplete, or Failed. Only Complete may update jobs and close missing source IDs.
- Each company persists independently. One failed, incomplete, or storage-failed company cannot invalidate another result.
- Blocking database, network, analytics, and discovery work stays outside presentation render loops.
- Direct search matches title or company. Skill discovery uses stored, evidence-linked extracted facts.
- Optional AI discovery uses bounded excerpts, strict validated output, cached attempts, safe local fallback, and explicit
  human approval before suggestions affect extraction.
- Jobs, history, application state, analytics, source health, and library data remain local by default.

## Issue execution

1. Read the assigned issue, its dependencies, acceptance criteria, and linked Rust references.
2. Confirm dependencies are merged into `era`; report a dependency conflict instead of expanding the issue.
3. Add or update the smallest deterministic test that proves parity or the requested behavior.
4. Implement only the assigned scope using the repository convention and target dependency direction.
5. Run focused checks, then every validation command named by the issue.
6. Update the issue ledger with evidence only when the orchestrator explicitly assigns that responsibility.

An issue is complete when every acceptance criterion is evidenced, required deterministic tests pass, architecture and
format checks pass, no unrelated files changed, and the resulting commit is small, conventional, and buildable.
