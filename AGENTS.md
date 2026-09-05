# TechJobsNL engineering

## Active goal

- **Goal:** While [`GOAL.md`](GOAL.md) exists, read it before planning, implementing, or reviewing migration work. It is the
  source of truth for the temporary mission and its completion conditions. The migration-closing issue removes it after
  every condition is accepted.

## Required context

- **Convention:** Before writing or reviewing .NET code, read and apply [`docs/DOTNET_CONVENTIONS.md`](docs/DOTNET_CONVENTIONS.md).
- **Migration policy:** While `GOAL.md` exists, read and apply
  [`docs/MIGRATION_POLICY.md`](docs/MIGRATION_POLICY.md) before migration planning, implementation, review, or orchestration.
- **Migration work:** Once the migration plan and issue ledger exist, read the issue named by the orchestrator and only the
  plan sections it points to.

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
