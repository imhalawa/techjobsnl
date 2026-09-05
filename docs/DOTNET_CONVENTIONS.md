# .NET and C# Conventions

This document defines the default engineering conventions for .NET projects in this repository. Build configuration,
EditorConfig, and analyzers are the executable source of truth. This guide owns semantic rules that tooling cannot fully
express.

## Baseline

- Target an actively supported .NET LTS release and its corresponding stable C# version.
- Pin the SDK, language version, and exact package versions in repository configuration.
- Use preview language or framework features only after an explicit project decision.
- Enable nullable reference types, deterministic builds, implicit framework usings, and warnings as errors.
- Enforce a curated analyzer ruleset. Review noisy rules individually rather than weakening a category globally.
- Centralize package versions and use locked restores in CI.

## Formatting and files

- Use four spaces, UTF-8, a final newline, and no trailing whitespace.
- Treat 120 characters as the normal line-width target. Keep an expression intact when wrapping would reduce clarity.
- Use file-scoped namespaces.
- Keep one primary top-level type in each file. Small private nested types may remain with their owner.
- Match namespaces exactly to the source folder hierarchy.
- Place `using` directives before the namespace in one alphabetically sorted group and remove unused imports.
- Keep global usings limited to framework namespaces.
- Require braces for every control-flow body.
- Use explicit accessibility except where the syntax already makes accessibility unambiguous.
- Order members as fields, constructors, properties, public methods, private methods, and nested types. Preserve cohesion
  when a different local order is clearer.
- Keep types navigable without organizational `#region` blocks.
- Prefer expression-bodied members whenever the complete member is one clear expression.
- Use `var` for local variables whenever the compiler permits it.

## Naming

- Use `PascalCase` for namespaces, types, members, and constants.
- Use `camelCase` for parameters and local variables.
- Use `_camelCase` for private instance fields.
- Prefix interfaces with `I`.
- Suffix task-returning methods with `Async`.
- Treat acronyms as words: `Http`, `Sqlite`, `Id`, and `Url`.
- Name tests `Behavior_Scenario_Result`.
- Give external contract models role-specific names such as `Request`, `Response`, or `Payload`.
- Suffix persistence records with `Row`, use plain names for canonical domain models, and suffix UI presentation models
  with `ViewModel`.
- Use role-specific model names instead of the term `DTO`.

## Language use

- Use stable modern C# features aggressively when they make intent shorter or clearer.
- Use traditional constructors. Do not introduce primary constructors.
- Prefer records for immutable models. Use classes for identity, lifecycle, framework behavior, or encapsulated behavior.
- Represent identifiers and constrained scalar values with validated `readonly record struct` value objects.
- Establish required valid state through constructors or named factories.
- Reserve `required init` properties for framework-bound configuration and serialization models.
- Use `with` for immutable data and state transitions only when it cannot bypass domain validation.
- Keep models, messages, results, and collections immutable.
- Prefer `ImmutableArray<T>` for ordered materialized results, immutable dictionaries or sets for keyed data, and frozen
  collections for long-lived lookup tables.
- Materialize collections before crossing a layer or ownership boundary. Do not expose lazy enumeration across such a
  boundary.
- Use LINQ for clear transformations. Prefer loops for complex control flow, streaming, diagnostics, or measured hot
  paths.
- Return named records for multi-value results. Do not return tuples.
- Replace hard-to-read parameter lists and boolean modes with immutable request or options records.
- Use extension methods for natural operations such as registration or mapping. Keep dependencies explicit.
- Permit `partial` only where a framework or source generator requires it. Keep handwritten implementations in one file.
- Use `async void` only for an unavoidable framework event signature and immediately delegate to task-returning code.
- Treat `unsafe`, `dynamic`, and reflection-heavy code as reviewed boundary exceptions requiring justification and tests.

## Nullability

- Enable nullable reference types for production and test code and treat diagnostics as errors.
- Model absence explicitly and validate it at the owning boundary.
- Use the null-forgiving operator only as a narrowly justified production exception.
- Never use the null-forgiving operator in tests.
- When a test receives a nullable value, assert non-null state with a flow-aware assertion and continue with the resulting
  non-null value.
- Declare nullable test values only when nullability is part of the behavior under test.

## APIs and dependencies

- Make implementations `internal` and `sealed` by default. Expose contracts deliberately and open inheritance only for a
  demonstrated extension point.
- Introduce interfaces at real boundaries such as persistence, external systems, time, filesystem, browser, and clipboard.
  Avoid one interface for every class.
- Use constructor injection and explicit registration modules. Keep service location and assembly scanning out of
  application code.
- Organize registration through focused extension methods owned by the registering module.
- Enforce domain invariants at creation boundaries.
- Use handwritten validation and typed validation errors.
- Represent expected domain and operational outcomes with small project-owned sealed record unions and exhaustive pattern
  matching.
- Reserve exceptions for unexpected faults.
- Catch an exception only to recover, translate a known boundary failure, add useful context, clean up, or handle an outer
  operation boundary.
- Preserve cancellation without translating it into an ordinary failure.
- Keep external, persistence, domain, and presentation models separate and map them with explicit pure methods.
- Keep disposable ownership lexical with `using` or `await using`. Do not dispose injected services.

## Async and concurrency

- Return `Task` or `Task<T>`. Use `ValueTask` only after measurement demonstrates a benefit.
- Accept `CancellationToken` on public asynchronous I/O and long-running operations, place it last, and propagate it.
- Require an explicit cancellation token below true application entry points.
- Use ordinary `await`. Add `ConfigureAwait(false)` only for a demonstrated library-context requirement.
- Track every background task, propagate cancellation, observe failures, and await shutdown.
- Prefer bounded `Channel<T>` streams with immutable messages for ordered progress crossing task or UI lifetimes.
- Keep mutable state owned by one component rather than exposing shared concurrent collections.
- Inject `TimeProvider` and narrow abstractions for environmental side effects.
- Use UTC `DateTimeOffset` for instants, `DateOnly` for dates, and `TimeSpan` for durations.
- Require explicit string comparison. Use ordinal comparison for identifiers and invariant culture for protocols,
  persistence, hashes, and logs.

## Logging and comments

- Use structured message templates, named properties, and operation scopes. Pass exceptions separately from message data.
- Give stable event IDs to lifecycle events, persistence outcomes, retries, configuration failures, and unexpected faults.
- Log operation summaries and meaningful state changes at information level. Keep per-item detail at debug or trace.
- Log a failure once at the boundary that handles it.
- Keep raw payloads, descriptions, secrets, credentials, and personal data out of logs. Prefer identifiers, counts, outcomes,
  and timings.
- Keep comments minimal. Explain intent, safety constraints, external contracts, or non-obvious tradeoffs rather than
  narrating syntax.
- Give every `TODO` or `FIXME` a concise reason.
- Apply analyzer or compiler suppressions at the narrowest possible location with a written justification.
- Document public contracts and non-obvious semantics with useful XML documentation.

## Testing

- Use xUnit v3, FluentAssertions 7.x, Moq, AutoFixture, and Verify.Xunit.
- Keep FluentAssertions on the final stable 7.x line unless a later licensing decision explicitly changes it.
- Use strict Moq mocks for true external boundaries. Verify meaningful interactions explicitly and avoid blanket
  `VerifyAll` calls.
- Use AutoFixture for incidental valid data. Freeze deliberate dependencies and spell out behavior-driving values.
- Structure unit tests as Arrange, Act, Assert.
- Structure integration tests as Given, When, Then.
- Give each test one behavioral reason to fail.
- Run independent unit tests in parallel. Isolate database, filesystem, and UI integration tests in controlled nonparallel
  collections.
- Allow incidental generated values to vary only when assertions cannot depend on them. Fix time, identifiers, ordering,
  and all behavior-driving input.
- Test UI commands and state transitions first. Use Verify.Xunit selectively for deterministic, reviewed presentation
  snapshots and scrub nondeterministic data explicitly.
- Tag tests as Unit, Integration, or Live. Run deterministic Unit and Integration tests by default; run Live tests only in
  an explicit external-contract job.
- Require explicit tests for critical behavior and an 80 percent line-coverage floor for each production project.
- Add architecture tests for dependency direction and project-specific structural invariants.

## Enforcement

- Use SDK analyzers and Meziantou.Analyzer with curated severities.
- Run locked restore, warnings-as-errors builds, `dotnet format --verify-no-changes`, tests, architecture checks, coverage,
  and supported-platform release builds in CI.
- Keep build configuration authoritative. Update this guide when an intentional semantic convention changes.
