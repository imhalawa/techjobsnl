# Target architecture

This document is the textual source of truth for the .NET project map, dependency direction, module ownership, interfaces,
adapters, and product invariants. [`architecture.mermaid`](architecture.mermaid) is its visual companion.

## Dependency direction

```text
TechJobsNL.App ----\
                    >---- TechJobsNL.Runtime ----> TechJobsNL.Core
TechJobsNL.Tui ----/                 |
                                   +----> adapter projects ----> TechJobsNL.Core
```

`TechJobsNL.Core` owns canonical behavior and every port interface. Presentation projects own their view and interaction
logic. Adapter projects implement Core-owned interfaces. `TechJobsNL.Runtime` is the composition root shared by executable
clients and contains wiring rather than business rules.

## Production projects

- `TechJobsNL.Core`: Domain, Application, canonical read models, and `Ports.*` interfaces.
- `TechJobsNL.Runtime`: host startup, configuration, logging, command/query registration, and explicit adapter composition.
- `TechJobsNL.App`: Avalonia desktop views, immutable ViewModel state, navigation, dialogs, and themes.
- `TechJobsNL.Tui`: terminal rendering, input, navigation, and presentation state.
- `TechJobsNL.Adapters.Providers`: company-directory, vacancy, official-publication, and trusted external-publication
  adapters.
- `TechJobsNL.Adapters.Analytics.Local`: deterministic extraction, technology analysis, coverage-aware Trends, and
  recommendations.
- `TechJobsNL.Adapters.AiExperience.DeepSeek`: optional DeepSeek implementation of AiExperience ports.
- `TechJobsNL.Persistence.Sqlite`: Dapper persistence adapter and explicit SQLite migrations.
- `TechJobsNL.Adapters.Platform`: operating-system secret storage, time, browser, clipboard, and platform actions.

Tests mirror a production project when they prove behavior through that project's interface. Acceptance tests may span
Runtime and a client while still asserting through public interfaces.

## Core modules and seams

Within Core, Domain depends on nothing. Application depends on Domain and owns commands, queries, progress, orchestration,
and canonical read models. Port interfaces live at the Application-owned seams:

- `Ports.Providers`: company directory, vacancies, official content, and trusted external publications.
- `Ports.Analytics`: fact extraction, technology analysis, Trends, and deterministic recommendations.
- `Ports.AiExperience`: summarization, filtering and ranking, cleanup, normalization, and interactive refinement.
- `Ports.Persistence`: companies, follows, evidence, content, Feed, profiles, library, settings, and history.
- `Ports.Platform`: protected secrets, time, browser, clipboard, and required platform behavior.

Core has no presentation, adapter, terminal, Avalonia, desktop, mobile, Dapper, Refit, Polly, or platform dependency.
Adapters depend on Core and not on presentation projects or one another. Add a port only where production and deterministic
test adapters make the seam real. Prefer deep modules: small interfaces that hide substantial behavior and concentrate
verification.

## Runtime and application interface

Runtime registers adapters explicitly. Project-owned command/query dispatch uses separate `ExecuteAsync` and `QueryAsync`
entry points and applies logging, timing, validation, then handler execution without MediatR, reflection, assembly scanning,
or hidden registration. Bounded channels carry ordered refresh and presentation progress.

App and Tui may present different views and interaction logic. They consume the same commands, queries, progress, canonical
read models, persistence behavior, analytics, and provider outcomes. They never depend on each other or duplicate business
rules.

## Adapter technology

- .NET 10 LTS and stable C# 14.
- Microsoft extensions for dependency injection, configuration, logging, and hosting.
- SQLite through Dapper and explicit lowercase SQL; repositories expose behavior rather than generic CRUD.
- Refit external contracts return `ApiResponse<T>` and use typed reflection-based System.Text.Json models.
- Named Polly resilience pipelines belong to the owning external adapter.
- Avalonia with CommunityToolkit.Mvvm; UI-required partial types remain in App and code-behind contains view mechanics only.
- Protected operating-system storage holds user secrets; secret values never enter SQLite, logs, diagnostics, or source
  control.

## Product invariants

- Startup reads local state and contacts no provider until the person explicitly requests external work.
- Every vacancy source uses an official company or ATS endpoint and yields stable identity and trusted official URLs.
- Provider results are Complete, Incomplete, or Failed. Only Complete may close missing source IDs.
- Each company commits independently; one failed, incomplete, or storage-failed company cannot invalidate another.
- Blocking persistence, network, analytics, and discovery work stays outside presentation render loops.
- Direct search matches title or company. Derived discovery uses retained, evidence-linked facts.
- Optional model behavior has bounded input, validated output, cached attempts, safe deterministic fallback, and explicit
  approval before changing canonical or preference state.
- Jobs, evidence, history, application state, analytics, source health, library data, and settings remain local by default.
