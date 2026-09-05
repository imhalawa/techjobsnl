# Release process

## Storage model

Unshipped planning lives only in the private GitHub Project governed by [`PLANNING_PRIVACY.md`](PLANNING_PRIVACY.md).
The public repository stores the shipped interface and public-safe shipped records:

```text
RELEASES.md
releases/
├── _template/
└── <version>/             # shipped releases only
```

`RELEASES.md` summarizes shipped versions and quality totals. Each `releases/<version>/` folder is created from `_template/`
only when that version ships and
contains:

- `CONTEXT.md`: domain terms introduced by that release;
- `architecture.mermaid`: delivered architecture delta;
- `release-notes.md`: behavior present in the published build;
- `bug-report.md`: defects attributed to the release;
- `prototype/README.md`: public-safe prototype decisions and approval evidence.

Private release promises, rejected designs, and task bodies remain in Project draft items after shipping.

## Lifecycle

The Project's release Lifecycle is `Draft`, `Planned`, `Current`, `Shipped`, or `Superseded`. Exactly zero or one release is
Current. Draft task decomposition may occur at any time. A task becomes Ready only when its release is Planned, applicable
prototype is Accepted, dependencies resolve, and every acceptance criterion has planned evidence. Only Ready tasks from the
Current release may be implemented.

Task bodies follow [`ISSUE_TEMPLATE.md`](ISSUE_TEMPLATE.md). They remain Project draft items through Task State `Done` and
archival.

## Shipping

A release becomes Shipped only when every promised journey works, acceptance evidence is approved, required checks pass,
upgrade and rollback behavior are proven, release notes match the build, defect totals are current, and every supported
platform starts the publishable artifact.

Shipping creates `releases/<version>/`, adds the version to `RELEASES.md`, and changes the private release draft's
Lifecycle to Shipped in one coordinated operation. Shipped release notes, architecture, context, and prototype decisions are
immutable. Corrections ship in a later patch release.

## Bug attribution

`bug-report.md` is the append-only exception to shipped-record immutability.

- Give every confirmed defect a stable `BUG-NNN` identifier.
- Attribute it to the release that introduced it, including defects found before publication.
- Record the detecting version and fixing version or pre-release commit.
- Mark Escaped Yes only when a shipped artifact exposed the defect to users.
- Keep uncertain attribution under the detecting release with `Introduced by` set to Unknown until root cause is known.
- Count duplicate reports once and link duplicates from the canonical row.
- Update the introducing release's bug report and root `RELEASES.md` totals together.

A defect discovered while a release is Current joins that release only when it blocks the promise or violates correctness,
security, privacy, data integrity, or a product invariant. Otherwise create a separate private task.
