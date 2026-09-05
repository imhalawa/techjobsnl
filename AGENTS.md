# TechJobsNL engineering

## Context on demand

- Start with the assigned task packet and its exact pointers; reuse documents already read this session.
- Migration: while `GOAL.md` exists, read it and `docs/MIGRATION_POLICY.md` once per session.
- .NET changes/review: read `docs/DOTNET_CONVENTIONS.md` once; revisit relevant sections as needed.
- Boundaries, ownership, persistence, composition, platform: read the applicable `docs/ARCHITECTURE.md` sections.
  Read `docs/architecture.mermaid` when changing the project graph, rather than for every adapter task.
- Product naming: read `CONTEXT.md`; add terms only for the Current release.
- GitHub planning access: read `docs/PLANNING_PRIVACY.md`; records remain private Project draft items.
- Release planning, activation, shipping, defect attribution: read `docs/RELEASE_PROCESS.md` and `RELEASES.md`.

## Execution

- Target `era`; preserve historical `main` and `archive/rust/`. Archive edits require an archive-integrity task.
- Implement one assigned Ready task from the Current release; verify dependency commits are integrated.
- Use the packet's exact scope, references, and checks. Widen context only for a named missing fact.
- Report contradictions, unsafe migrations, and newly discovered scope instead of silently expanding the task.
- `Workflow.md` owns the short execution loop; `docs/ISSUE_TEMPLATE.md` owns task authoring.
- Finish with acceptance evidence, validation results, changed files, compatibility/risk, follow-ups if any,
  and conventional commit hash.
