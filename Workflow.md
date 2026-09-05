# Sequential v0.1.0 migration orchestrator

Use the following instructions as the complete prompt for the development orchestrator.

## Objective

Implement every private `v0.1.0` migration task in the TechJobsNL Roadmap, strictly in ascending `Sequence` order, using
one implementation sub-agent at a time. After each implementation is committed and pushed to `era`, run an independent
`gpt-5.6-sol` High review. Advance only after that review accepts the complete task result.

Finish after `V0.1.0-059`. Report that Core is ready for the separately drafted Avalonia wave, but do not draft or
implement Avalonia tasks in this run.

## Sources of truth

- Repository: `imhalawa/techjobsnl`.
- Migration branch and direct push target: `era`.
- Private GitHub Project: `imhalawa/6`, **TechJobsNL Roadmap**.
- Release: `v0.1.0`.
- Work queue: private Project draft items whose `Record Type` is `Task` and `Release` is `v0.1.0`.
- Order: numeric `Sequence`, currently `V0.1.0-001` through `V0.1.0-059`.
- Task contract: `docs/ISSUE_TEMPLATE.md`.
- Repository instructions: `AGENTS.md` and every context file it requires for the active task or review.

Treat the Project fields and each Project draft item's body as authoritative. Keep every planning record a Project draft
item. Never convert it to a repository issue, create a public issue from it, copy its private body into a public artifact,
or change Project visibility or access.

## Non-negotiable execution policy

- Execute sequentially. At most one implementation sub-agent or review sub-agent may be running.
- Use the task's exact `Minimum Model` and `Reasoning Effort` for every implementation and corrective pass.
- Use `gpt-5.6-sol` with **High** reasoning for every independent review. This review-only override is intentional; it does
  not change the task's stored minimum implementation model.
- Work directly on local `era` and push directly to `origin/era`. Do not create task branches, worktrees, or pull requests.
- Each implementation sub-agent writes a short, one-line Conventional Commit subject ending with the stable task ID, for
  example `feat(core): port vacancy identity (V0.1.0-003)`.
- Preserve history. Use additive corrective commits after a rejected review. Never force-push, rewrite published commits,
  reset destructively, or alter `archive/rust/` unless the active task explicitly owns archive integrity.
- Keep the worktree clean between agents. Never begin the next task while the current task is unreviewed, rejected,
  blocked, uncommitted, or unpushed.
- A task's scope, exclusions, acceptance criteria, Rust references, expected change surface, validation commands, and stop
  conditions bind both implementer and reviewer.

## Initialize once

1. Read `AGENTS.md`, `GOAL.md`, `docs/MIGRATION_POLICY.md`, `docs/PLANNING_PRIVACY.md`, `docs/ISSUE_TEMPLATE.md`,
   `docs/DOTNET_CONVENTIONS.md`, and `docs/ARCHITECTURE.md` completely. Read other pointed context only when the active task
   triggers it.
2. Verify the Project is private and the queue contains Project draft items, not repository issues.
3. Verify the queue has unique task IDs and sequences, every dependency resolves inside `v0.1.0`, and the dependency graph
   is acyclic. Stop on any discrepancy.
4. Fetch `origin`, switch to `era`, fast-forward to `origin/era`, and verify a clean worktree. Stop rather than discard or
   overwrite unexpected local changes.
5. Record the starting `era` commit. Set the v0.1.0 release `Lifecycle` to `Current` when implementation begins. Do not mark
   the release `Shipped` in this workflow.

Initialization is complete only when the private queue, clean branch, remote alignment, and dependency graph are all
verified.

## Main loop

For each task in ascending `Sequence` order:

### 1. Establish readiness

1. Reload the Project draft item and all its fields; do not rely on an earlier cached body.
2. If `Task State` is `Done`, verify its accepted commit is contained in `origin/era`, record the skip, and continue.
3. If `Task State` is `Removed`, record the intentional skip and continue.
4. Verify every task in `Depends On` is `Done` and its accepted commit is contained in `origin/era`.
5. Verify local `era` equals `origin/era` and the worktree is clean.
6. Change the task to `Task State: Ready` and built-in `Status: Todo`, then immediately to `Task State: Active` and
   `Status: In Progress` when its implementation sub-agent starts.

Readiness is complete only when all dependencies and the branch are verified. A later-sequence dependency, missing commit,
dirty worktree, changed task body, or failed stop condition blocks the loop; report it instead of improvising.

### 2. Dispatch exactly one implementation sub-agent

Start one sub-agent using the task's exact `Minimum Model` and `Reasoning Effort`. Give it:

- the complete current Project draft item body and fields;
- the current `era` commit as its base;
- the repository path and direct-push authorization for `origin/era`;
- an instruction to read `AGENTS.md` and every task-required reference itself;
- an instruction to implement only the task's independently reviewable outcome;
- an instruction to run every named validation command and the smallest relevant focused checks;
- an instruction to leave no generated, temporary, unrelated, or untracked files;
- an instruction to make a short Conventional Commit and push it directly to `origin/era`;
- an instruction to return the commit SHA, acceptance evidence, exact validation results, changed files, compatibility
  impact, remaining risks, and follow-up proposals.

The implementation sub-agent owns implementation, validation, commit, and push. The orchestrator does not silently repair
or complete its code.

Wait for the sub-agent. If it invokes a task stop condition or cannot safely commit and push, set `Task State: Blocked`,
keep `Status: In Progress`, record the exact blocker, and stop the workflow for human direction.

### 3. Verify the pushed handoff

After the implementation sub-agent reports completion:

1. Fetch `origin/era` and verify the reported commit is present and descends from the recorded pre-task base.
2. Verify local `era` fast-forwards to the remote and the worktree is clean.
3. Reject the handoff immediately if the commit is missing, history was rewritten, unrelated files remain, the archive
   changed outside scope, or the commit subject is not a short Conventional Commit.
4. Record `pre-task SHA..current SHA` as the cumulative review range.
5. Change the task to `Task State: Review` and keep built-in `Status: In Progress`.

The handoff is complete only when the pushed cumulative range is immutable, clean, and reproducible locally.

### 4. Dispatch the independent Sol High reviewer

Start a new `gpt-5.6-sol` sub-agent with **High** reasoning. It is read-only: it reviews and reports; it does not edit,
commit, push, or change Project fields.

Give the reviewer:

- the complete current task body and Project fields;
- `AGENTS.md` and every task-required reference;
- the pre-task base SHA, current `origin/era` SHA, and cumulative diff range;
- the implementer's completion report and validation evidence;
- the instruction to inspect the actual diff and repository state rather than trusting the report.

The reviewer must evaluate both axes:

1. **Specification:** every acceptance criterion, Rust parity requirement, invariant, dependency, scope inclusion,
   exclusion, compatibility promise, validation command, and stop condition.
2. **Standards:** .NET conventions, architecture direction, module/interface/seam ownership, test quality, determinism,
   source safety, privacy, local-first behavior, migration reversibility, commit hygiene, and absence of unrelated changes.

It must rerun proportionate focused validation and inspect failures and negative paths. It must return exactly one verdict:

- `ACCEPT` — followed by criterion-by-criterion evidence and any non-blocking observations; or
- `REJECT` — followed by a numbered list of blocking findings. Each finding names severity, violated criterion or guideline,
  exact file/location or missing evidence, why it matters, and the smallest acceptable correction.

Acceptance requires zero blocking findings and evidence for every acceptance criterion. A plausible implementation, passing
subset, or clean diff without specification proof is not acceptance.

### 5. Correct rejected work

When the reviewer returns `REJECT`:

1. Change the task back to `Task State: Active`; keep `Status: In Progress`.
2. Reload the task and verify `origin/era` has not moved unexpectedly.
3. Start one corrective sub-agent using the task's original implementation model and reasoning effort. Give it the full
   task, cumulative range, review findings, and direct-push rules.
4. Require the smallest in-scope correction, focused regression tests for every finding, full named validation, a new short
   Conventional Commit, and a direct push to `origin/era`.
5. Preserve prior commits. Expand the cumulative review range from the original pre-task SHA through the newest pushed SHA.
6. Return to the independent Sol High review step with a fresh reviewer sub-agent.

Repeat correction and review until `ACCEPT` or a task stop condition genuinely blocks progress. There is no arbitrary retry
limit, and reviewer findings never authorize expansion beyond the task. Out-of-scope needs become private follow-up
proposals for human triage.

### 6. Complete the accepted task

After `ACCEPT`:

1. Verify the accepted current SHA is still the head of `origin/era`, is locally reproducible, and the worktree is clean.
2. Add the implementation and corrective commit SHAs, reviewer verdict, acceptance evidence, validation results, changed
   files, compatibility impact, risks, and follow-up proposals to the private Project draft item's completion report.
3. Set `Task State: Done` and built-in `Status: Done`.
4. Record the accepted SHA as the base for the next task.
5. Reload the next task from the Project and repeat the loop.

The task is complete only after the pushed cumulative result is independently accepted and its private record is Done.

## Remote movement and recovery

- If `origin/era` moves between steps, fetch and identify the exact commits before acting.
- Fast-forward when the movement is an already accepted workflow commit. When it is unexpected, stop and report the
  conflicting SHA and task rather than merging, rebasing, force-pushing, or guessing ownership.
- If a sub-agent leaves a dirty worktree, return control to a corrective sub-agent for the same task. Preserve user-owned
  changes and never clean them destructively.
- If GitHub Project access, authentication, pushing, or required validation is unavailable, keep the active task Blocked
  with precise evidence and stop.

## Workflow completion

After `V0.1.0-059` is independently accepted and marked Done:

1. Verify all `V0.1.0-001` through `V0.1.0-059` records are Done or explicitly Removed, all accepted SHAs are ancestors of
   `origin/era`, and no task remains Active, Review, Ready, or Blocked.
2. Run the cumulative repository checks named by `V0.1.0-059` and verify the worktree is clean.
3. Leave the release `Current`; v0.1.0 is not shipped until the later Avalonia wave is implemented and accepted.
4. Produce a concise final report containing completed/skipped task totals, final `era` SHA, review/fix counts, validation
   results, remaining risks, and confirmation that Avalonia tasks were neither drafted nor implemented.
5. Stop. Await explicit human approval to draft the Avalonia issue wave.
