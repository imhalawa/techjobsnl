# Migration task contract

Use this contract for every private task Project draft item. One task owns one independently reviewable outcome and normally
produces one conventional commit.

Populate these Project fields with the same values as the body: Record Type `Task`, Release, Task ID, Sequence, Task State,
Kind, Area, Risk, Minimum Model, Reasoning Effort, and Depends On.

## Title

`[v<version>] <imperative, outcome-focused title>`

## Body

```markdown
## Identity

| Field | Value |
| --- | --- |
| Task ID | `V<version>-NNN` |
| Release | `v<version>` |
| Kind | Foundation / Feature / Adapter / Migration / UX / Test / Documentation / Bug |
| Primary area | Core / App / Tui / Runtime / Providers / Analytics / AiExperience / Persistence / Platform / Build |
| Risk | Low / Medium / High / Critical |
| Minimum model | `gpt-5.6-luna` / `gpt-5.6-terra` / `gpt-5.6-sol` |
| Reasoning effort | Low / Medium |
| Depends on | `None` or stable task IDs |
| Can run in parallel with | `None` or stable task IDs |

## Outcome

<One paragraph describing the independently observable result.>

## User or system value

<Why this outcome advances the release promise.>

## Required context

- `AGENTS.md`
- Private release Project draft item: <title and specific sections>
- Private task Project draft item: <stable task ID>
- `docs/ARCHITECTURE.md`: <specific sections>
- `docs/architecture.mermaid`: <specific nodes or flows>
- `docs/DOTNET_CONVENTIONS.md`
- `docs/MIGRATION_POLICY.md`
- Rust implementation: <exact files>
- Rust tests or fixtures: <exact files>

## Dependencies

- Required merged tasks: <task IDs or None>
- Required interfaces or artifacts: <exact contracts>
- Readiness evidence: <how to verify dependencies>

## In scope

- <owned behavior>
- <owned interface or adapter>
- <owned deterministic tests>
- <owned documentation or migration artifact>

## Excluded

- <adjacent behavior owned elsewhere>
- <future-release concern>
- <unrequired refactor>

## Design and implementation guidance

### Ownership

- Owning project and module: <name>
- Interface and seam: <Core-owned contract>
- Adapter: <implementation project or None>
- Consumers: <named callers>

### Required behavior

- <domain rule or state transition>
- <failure behavior>
- <ordering or concurrency behavior when applicable>

### Compatibility and safety

| Concern | Requirement |
| --- | --- |
| Existing SQLite data | <impact, migration, or None> |
| Existing TOML configuration | <impact, migration, or None> |
| Network behavior | <when contact is permitted or None> |
| Source trust | <identity, completeness, and URL rules or None> |
| Privacy and secrets | <requirements or None> |
| Recovery and rollback | <required path or None> |

## Expected change surface

- <project or file family>
- <test or fixture family>
- <documentation artifact when required>

## Acceptance criteria

- [ ] **AC-1:** <observable binary condition>
- [ ] **AC-2:** <observable binary condition>
- [ ] **AC-3:** <failure or safety condition>
- [ ] **AC-4:** <persistence, restart, parity, or journey condition when applicable>

## Planned evidence

| Criterion | Evidence |
| --- | --- |
| AC-1 | <test, artifact, or reproducible inspection> |
| AC-2 | <test, artifact, or reproducible inspection> |
| AC-3 | <negative-path test> |
| AC-4 | <migration, restart, parity, or journey test> |

## Validation commands

1. <smallest focused test>
2. <owning-project tests>
3. <format, analyzer, and architecture checks>
4. <release-required validation>

## Completion report

- Acceptance criteria and evidence
- Commands and exact results
- Files changed
- Compatibility or migration impact
- Remaining risks
- Follow-up task proposals
- Commit hash

## Stop conditions

- A dependency is not merged into `era`.
- Rust behavior contradicts the task or its references.
- An acceptance criterion requires another release's scope.
- Safe data compatibility or rollback cannot be demonstrated.
- The implementation requires a new dependency direction.
- Required credentials or a human product decision are missing.
```

Every heading is required. Write `None` where a branch does not apply rather than removing it. Acceptance criteria state
observable results; implementation activity belongs under guidance or expected change surface.

## Sizing gate

Split a task when it contains more than one independently useful outcome, multiple provider strategies beyond one shared
provider contract, both a reusable contract and several production adapters, unrelated schema and UI changes, or more than
one independently reviewable migration. A vertical slice may cross Core, persistence, Runtime, and App only when every
change proves one small user journey.

## Model assignment

Use the lowest sufficient tier. Terra Medium is the default; assigning Sol requires a task-specific reason.

| Tier | Use for | Model | Effort |
| --- | --- | --- | --- |
| Mechanical | Documentation, boilerplate, fixture registration, cleanup, or a tightly specified isolated change | `gpt-5.6-luna` | Low |
| Implementation | Rust translation, adapters against established interfaces, deterministic tests, normal Core/App work, persistence, or a bounded vertical slice | `gpt-5.6-terra` | Medium |
| Lead | Cross-project coordination, unresolved Rust contradictions, risky migration/cutover work, or final integration review | `gpt-5.6-sol` | Medium |

Sol Medium is the ceiling for planned migration work. Split a task whose uncertainty appears to require more reasoning rather
than escalating the model or effort.

## Workflow mapping

| Task State | Built-in Status | Meaning |
| --- | --- | --- |
| Draft | Todo | Decomposition is still being reviewed. |
| Ready | Todo | All readiness gates pass; waiting for the Current release. |
| Active | In Progress | One agent owns implementation. |
| Review | In Progress | Implementation is complete and evidence is under review. |
| Blocked | In Progress | A named stop condition prevents progress. |
| Done | Done | Acceptance evidence is approved and the commit is integrated. |
| Removed | Done | Retained planning record; intentionally not implemented. |
