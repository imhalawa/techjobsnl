# Current architecture

This document describes the projects present in the working solution. [`architecture.mermaid`](architecture.mermaid)
is its visual companion. Add projects when an assigned delivery needs them; future delivery plans and ownership remain
in the private Project. Removing unused scaffolds does not remove the migration obligations in `GOAL.md`.

## Dependency direction

```text
TechJobsNL.App --> TechJobsNL.Runtime --> TechJobsNL.Core
                         |
                         +--> TechJobsNL.Adapters.Providers --> TechJobsNL.Core
                         +--> TechJobsNL.Persistence.Sqlite --> TechJobsNL.Core
```

Core owns canonical behavior and port interfaces. Adapters implement Core contracts. Runtime owns explicit composition;
presentation consumes Runtime and the Core application interface. Core has no presentation, adapter, platform, Dapper,
Refit, Polly, or Avalonia dependencies.

## Projects and folders

`TechJobsNL.slnx` groups production projects under `src` and matching test projects under `tests`, matching the filesystem.

- `TechJobsNL.Core`: domain/configuration models, dispatch, progress, scan orchestration, eligibility, vacancy queries,
  company projections, operational read models, and persistence/provider boundaries.
- `TechJobsNL.Runtime`: configuration loading, startup and resource ownership, explicit query registration and composition.
- `TechJobsNL.App`: desktop executable and its presentation behavior. The executable currently remains a host scaffold.
- `TechJobsNL.Adapters.Providers`: external vacancy contracts, safe HTTP behavior, and source normalization.
- `TechJobsNL.Persistence.Sqlite`: compatible SQLite opening/recovery, local persistence and read adapters.

Tests mirror the owning production project. Runtime integration tests may span Core and adapters while asserting through
the public Runtime/Core interface. Do not retain empty projects solely to reserve a future name or dependency.

## Core boundaries

Domain models have no environmental dependencies. Canonical application behavior owns commands, queries, progress,
orchestration and read models. Port interfaces belong to Core and hide persistence, provider and platform details.
Add a port where a real adapter makes the boundary useful; avoid generic CRUD and speculative extension points.

Keep collections immutable across ownership boundaries. Preserve company/source identity and source outcome semantics.
Presentation state and interaction logic belong to the client; search, eligibility, lifecycle and retained-data meaning
belong to Core.

## Runtime and application interface

Register adapters explicitly. Project-owned dispatch uses separate `ExecuteAsync` and `QueryAsync` entry points, with
logging, timing, validation and handler execution. Avoid service location, assembly scanning and hidden registration.
Bounded channels carry ordered scan progress across lifetimes.

Local browsing loads TOML and compatible SQLite state off the presentation thread, materializes a Core catalogue and
closes database handles before returning a session. That session queries an immutable snapshot; reopening reloads local
changes. Startup reports configuration, database, recovery and retained-data failures distinctly. It does not register
or call external providers. Disposing a browsing session releases its snapshot and rejects subsequent queries.

## Adapter technology

- .NET 10 LTS and stable C# 14, pinned by repository build configuration.
- Microsoft extensions for hosting, dependency injection and logging at composition boundaries.
- SQLite through Dapper with explicit lowercase SQL and compatible, recoverable migrations.
- Refit contracts and owning-adapter resilience for external source behavior.
- Avalonia and CommunityToolkit.Mvvm for desktop presentation.

## Product invariants

- Startup reads local state and contacts no source until the person explicitly requests external work.
- Sources use official company or ATS endpoints, stable identity and trusted official URLs.
- Complete, Incomplete and Failed remain distinct. Only Complete can close missing source IDs.
- Each company commits independently; failure retains that company's trusted vacancies.
- Blocking persistence/network work stays outside presentation render loops.
- Direct search matches title or company through shared Core behavior.
- Existing TOML, SQLite data and deferred stored values remain compatible; preserve recovery backups.
- Operational views omit raw payloads and redact sensitive diagnostic components.
- Secrets belong in protected storage, not logs, SQLite diagnostics or source control.
