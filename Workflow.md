# Deliver one small task

The private Project owns the queue. `docs/RELEASE_PROCESS.md` owns release gates;
`docs/ISSUE_TEMPLATE.md` owns task shape. This replaces the sequential migration orchestrator.

Every unfinished requirement has a visible task and owning release. Draft means not yet executable, not hidden.
Archive unfinished records only after visible replacements take ownership of all scope and acceptance criteria.

1. Read `AGENTS.md`, the assigned packet, and its exact context pointers. Load the Current release promise once.
   Reuse already-read context. Fetch dependency states and commit hashes, not their complete task bodies.
2. Verify the task is Ready in the Current release and dependency commits are integrated into `era`.
   Inspect the named files/tests. Expand the search only for a specific unresolved question.
3. Mark Active. Implement and test the one outcome. A task may cross layers to deliver one small journey.
4. Run the packet's focused checks, affected-project checks, and `git diff --check`.
   Reuse successful evidence for the same code; repeat checks after relevant changes or failures.
5. Review the diff against acceptance criteria. Use independent review when the packet requires it or when
   data migration, source-completeness changes, or unresolved correctness risks warrant it.
6. Commit conventionally and integrate using the session-authorized git workflow. Record evidence and hash
   in the private task. Done means validated and integrated, with any required independent review accepted.
7. Stop after the assigned task unless the user requested a bounded batch. No automatic whole-roadmap run.

Use dependency order, not numeric sequence. Default to one implementation task at a time. Independent tasks
may use isolated worktrees when authorized. Preserve concurrent work and never reset a dirty worktree.

Aim for roughly 30–60 minutes of implementation and focused verification per task. This is a sizing signal,
not a timeout or permission to omit requirements. If investigation exposes a larger outcome, capture the
specific unknown and split before expanding implementation. A planning change does not interrupt an Active
task or silently change its acceptance criteria; preserve its owner's completion or explicit handoff.

At release acceptance, run the full required checks once on the candidate and inspect the user journey.
Existing task-specific validation and review obligations remain binding until explicitly transferred to a
named acceptance task in the private traceability record. Sequence/gate changes do not erase evidence duties.

Report: outcome, acceptance evidence, exact validation results, changed files, compatibility/risk, follow-ups
if any, and commit hash. Link evidence rather than copying logs. Replanning itself changes no application code.
