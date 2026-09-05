# TechJobsNL Windows candidate

This portable Windows x64 build browses vacancies already retained on your device. Extract or copy the entire folder,
then run `TechJobsNL.App.exe`. The .NET runtime is included; installing an SDK or runtime is unnecessary.

The current candidate supports title/company search, keyboard selection, and read-only vacancy details.
It does not yet run scans or provide the full migrated desktop workflow. It does not contact vacancy providers.

## Local data

Normal startup uses `%APPDATA%\techjobsnl\config.toml`. If no configuration exists, compatible defaults are created.
The configured database path is resolved relative to that configuration folder. A new database displays an empty list.
To inspect a copy of existing data, pass its configuration path as the first argument:

```powershell
.\TechJobsNL.App.exe 'C:\My vacancy copy\config.toml'
```

Keep a copy of both your configuration and database before trying a candidate. Copy them while other TechJobsNL instances
are closed. Point the copied configuration at the copied database, particularly if its database path was absolute.
The candidate preserves existing settings and uses the tested SQLite migration/backup path when an upgrade is needed.

## Browse

Type a title or company in the search box. Description text is not searched. Up/Down moves selection while search retains
focus; Escape clears the search. Tab moves between controls. The detail pane shows stored text and an official vacancy
URL that can be selected and copied. It does not open URLs or apply for jobs.

Search uses one local snapshot per window. Close and reopen to see data changed by another instance.
If search fails, previous trusted results remain visible and are explicitly labelled.

## Recovery

If configuration is invalid or the database cannot be opened, the window displays an error. Close it, correct the copied
configuration or restore your known-good copy, and reopen. Do not delete the original database to resolve an error.
An unsuccessful supported migration restores the original data and retains a backup beside the database. Preserve that
backup until the candidate is accepted. A corrupt or unsupported database is reported rather than replaced.

## Build provenance

`manifest.json` records the source revision, whether the source tree was modified, the target runtime, and SHA-256 hashes
for the packaged files. This is an unsigned evaluation candidate. Cross-platform packaging and the full desktop migration
remain unverified; this document does not declare a shipped release.
