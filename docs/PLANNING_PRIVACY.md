# Planning privacy

This policy separates private planning from public repository work.

## Records

- A **Project draft item** is a GitHub-native draft record in the private
  [`TechJobsNL Roadmap`](https://github.com/users/imhalawa/projects/6). Planned and Current releases, prototypes, product
  ideas, and migration tasks live there.
- A **repository issue** belongs to a repository and inherits that repository's visibility. An issue in the public
  `imhalawa/techjobsnl` repository is public.
- Completion, Status `Done`, and Project archival preserve a Project draft item. Conversion into a repository issue is a
  separate publication action.

Use the exact terms **Project draft item** and **repository issue** whenever creation, visibility, completion, or archival
is involved.

## Workflow

1. Treat the private Project as the source of truth for every unshipped release and task.
2. Give every task a stable release-scoped ID such as `V0.1.0-001`; use it in Project fields, branches, commits, pull
   requests, and cross-references.
3. Keep task bodies, prototypes, roadmap ideas, and unreleased architecture in Project draft items or another explicitly
   approved private workspace.
4. Move draft items through their Project fields. Completion means Task State `Done`; archive the item when useful.
5. Publish only implementation evidence required by the public code change. A public commit or pull request may cite the
   stable task ID without copying its private body.
6. When a release ships, create its public-safe record under `releases/<version>/` from the repository template.

## Authorization boundary

Keep Project `imhalawa/6` private and every planning record a Project draft item for its entire lifecycle. Converting a
draft, creating a repository issue, changing Project visibility, granting Project access, or publishing private content
requires the user's explicit request for that action at that time. Permission to plan, implement, complete, archive,
commit, push, or open a pull request does not grant publication permission.

Before a GitHub mutation, verify the target object type and destination. Stop before the mutation when either is ambiguous.
