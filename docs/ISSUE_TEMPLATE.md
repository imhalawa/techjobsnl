# Small task packet

Aim for 200–400 words and one observable outcome. Fields own identity/state; the body owns behavior/evidence.
Required execution metadata: stable Task ID, delivery release, Task State, dependencies and integrated hashes.
Preserve existing IDs and historical fields. The private Project README identifies the current delivery field.

```markdown
## Outcome
One user-visible result, or a small prerequisite for that result.

## Change
Exact expected files/modules. One sentence of important exclusions.

## Context
Exact interfaces, reference files/test cases, and relevant policy sections. Read only these initially.

## Accept
- Observable success condition → named test or inspection.
- Relevant failure/data-preservation condition → negative-path evidence.
- Restart/integration condition when applicable → evidence.

## Check
Exact focused commands, affected-project checks, and required manual checks.

## Evidence
On completion: result, validation, integrated commit, required review, material risk or None.
```

Use two to four criteria in new packets. Preserve every inherited acceptance criterion verbatim in its source
record and map it to a destination; split broad criteria into verifiable sub-obligations without weakening them.
The packet includes the exact inherited obligations it executes. Executors need not open the whole legacy task.
Non-AC scope, commands, artifacts, invariants, and review duties also need an owner; an AC-only mapping is insufficient.

Detail only Current-release work. Future release sketches retain source references and requirements without
speculative task decomposition. Split multiple useful outcomes, multiple new source strategies, or unresolved
design decisions. A short investigation answers one named unknown and produces a decision or repro.

Draft: scope/evidence not ready. Ready: concrete dependencies integrated and checks specified. Active: owned.
Review: awaiting required review. Blocked: names missing prerequisite. Done: validated/integrated with required
reviews. Removed: superseded or intentionally dropped, with traceability. Archival preserves private records.
Built-in Status: Todo for Draft/Ready; In Progress for Active/Review/Blocked; Done for Done/Removed.

Use the session's configured model unless a packet states a justified capability requirement. Routine work
does not require a separate reviewer session. Retained explicit review obligations remain binding.
