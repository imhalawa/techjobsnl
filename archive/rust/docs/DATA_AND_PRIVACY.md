# Data and privacy

Jobs and personal state remain local. Network requests occur only for scans, opened vacancy URLs, and enabled provider discovery.

## What is stored

The SQLite database stores:

- normalized job records and original source payloads;
- job descriptions, official URLs, locations, publication dates, and lifecycle dates;
- complete, incomplete, and failed scan history with diagnostics;
- applied markers and closed/reopened state;
- versioned content snapshots and cached analytics facts;
- Analytics filters, starred Library items, skill status, and target roles;
- optional emerging-skill suggestions and their review status.

Default database path: `.data/techjobsnl.sqlite3`, relative to the user configuration directory.

## Network behavior

- Startup loads stored data and does not scan automatically.
- Pressing `r` contacts the official endpoints of enabled companies.
- Opening a job uses its official URL in the system browser.
- No analytics provider is required; `provider = "local"` makes no AI CLI call.

TechJobsNL does not prove that every external careers endpoint has the same privacy policy. Review the companies and URLs in [SOURCES.md](../SOURCES.md) if that matters for your environment.

## Optional Claude or Codex discovery

When `analytics.provider` is `claude` or `codex`, TechJobsNL may invoke an installed and authenticated local CLI when Analytics is opened.

The app:

- sends bounded excerpts from locally stored job descriptions;
- disables Claude tools or gives Codex an isolated empty working directory;
- requires strict JSON output;
- validates suggestions against exact supplied posting text;
- caches each attempt;
- falls back safely to the local bank on missing executables, timeout, failure, or invalid output;
- places suggestions in Library → Skills for explicit approval or rejection;
- never changes analytics automatically from an unreviewed suggestion.

Your CLI provider may have its own account, retention, and privacy terms. Do not enable it if you do not want posting excerpts sent through that provider.

## Backup

Close TechJobsNL, then back up both:

1. `config.toml`
2. `.data/techjobsnl.sqlite3`

Vacancies can be rescanned. The backup preserves configuration, applied markers, analytics state, Library choices, and history that sources may no longer expose.

## Reset or delete local data

Close TechJobsNL first, then delete the SQLite database if you want a full data reset. This permanently removes job history, snapshots, scan diagnostics, applied markers, analytics state, and library choices.

The next run creates an empty database. The next complete scan treats every eligible observed job as new because no earlier lifecycle exists.

Deleting only `config.toml` loses user settings; the next run recreates shipped defaults. Do not delete either file while the app is running.

## Legal-employer and sponsor caveat

A vacancy usually identifies a company brand, not necessarily the legal entity that will sign the employment contract. Even when a matching entity appears in the Dutch IND sponsor register, that mapping can be an inference [reasoned link], not vacancy-level proof.

Confirm the eventual employment entity and current sponsor status before relying on it. [SOURCES.md](../SOURCES.md) keeps the evidence and uncertainty separate for this reason.
