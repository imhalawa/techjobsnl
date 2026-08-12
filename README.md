# Job Watch

Job Watch is a local terminal application for reviewing eligible Netherlands vacancies. This foundation release enables Mollie's Ashby board only.

## Run

From the repository root:

```bash
cd tools/job-watch && cargo run --release
```

The application reads `config.toml` from the current directory. It loads stored active jobs at startup and does not contact a source until you press `r`.

## Keys

- `r`: scan enabled sources
- Arrow keys or `j`/`k`: move through jobs and views
- `J`/`K`: scroll job details
- `/`: search by job title or company; `Esc` clears the search
- `f`: cycle company, new, and applied filters, then clear the filter
- `h`: switch between active jobs and history
- `a`: mark or unmark the selected job as applied
- `o`: open the selected job in the default browser
- `?`: show or hide help
- `q`: quit

## Local data

The SQLite database is `.data/job-watch.sqlite3`, relative to this directory. It stores jobs, scan history, lifecycle changes, and applied status.

It is safe to delete the database while Job Watch is not running. Deletion permanently removes all local history and applied markers; the next run creates an empty database, and the next complete scan treats every eligible job as new.

## Scan state

- A complete company scan updates observed jobs and closes source IDs absent from that complete result.
- An incomplete company scan records diagnostics but does not add, update, or close jobs.
- A failed company scan records the failure but does not add, update, or close jobs.

Companies are isolated: one failed or incomplete company does not prevent a complete result from another company being stored.

## Tests

Run the offline suite, including the fake-source end-to-end lifecycle:

```bash
cargo test --all-targets
```

Run the ignored live Ashby smoke test separately (network access required):

```bash
cargo test --test ashby_test -- --ignored
```

The live smoke test checks Mollie's current public Ashby payload. The offline suite is the deterministic verification path.

## Company onboarding

Adyen, Booking.com, and the remaining allowlist are not enabled in this foundation release. Each requires a separate onboarding plan covering its source contract, location evidence, completeness rules, fixtures, and live verification before it can be added safely.
