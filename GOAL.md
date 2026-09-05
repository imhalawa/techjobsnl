# Active Goal: Rust-to-.NET Migration

Replace TechJobsNL's Rust implementation with a C#/.NET implementation while preserving observable behavior, existing
local data, source-safety guarantees, and the terminal workflow. Add an Avalonia desktop client and keep Core portable
for later Android and iOS clients.

## Completion conditions

The migration succeeds when:

- `TechJobsNL.Core` contains the shared product behavior without presentation or platform dependencies.
- The .NET terminal client preserves the supported Rust terminal workflows.
- The Avalonia desktop client exposes the agreed product features through the shared Core.
- Existing TOML configuration and SQLite data remain compatible or have tested, reversible migrations.
- Complete, Incomplete, and Failed source outcomes preserve the Rust safety semantics.
- Critical behavior has deterministic parity coverage.
- Supported Windows, Linux, and macOS builds and tests pass.
- The .NET implementation is accepted as the primary release.

## Lifecycle

This is a temporary mission file. The final migration-closing task must delete `GOAL.md` after every completion condition
has evidence and approval. That task must also replace migration-only instructions in `AGENTS.md` with the enduring .NET
project guidance.
