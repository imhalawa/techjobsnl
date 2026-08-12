# Job Watch

Job Watch is a local terminal application for reviewing eligible Netherlands vacancies. It enables Mollie's and Airwallex's Ashby boards, Booking.com's Jibe API, and eBay's official Netherlands careers pages.

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
- `/`: search by job title or company; `Enter` accepts the search, while `Esc` cancels and clears it during editing (press `/`, then `Esc`, to clear an accepted search)
- `f`: cycle company, new, and applied filters, then clear the filter
- `h`: switch between active jobs and history
- `a`: mark or unmark the selected job as applied
- `o`: open the selected job in the default browser
- `c`: copy the selected job URL to the system clipboard
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

Run the ignored live source smoke tests separately (network access required):

```bash
cargo test --test ashby_test -- --ignored
cargo test --test hosted_ats_test jibe_live_returns_complete_unique_jobs -- --ignored --nocapture
cargo test --test ebay_test ebay_live_returns_complete_unique_netherlands_jobs -- --ignored --nocapture
```

The live smoke tests check Mollie's, Airwallex's, Booking.com's, and eBay's current public payloads. The offline suite is the deterministic verification path.

## Company onboarding

Rabobank is tracked but disabled. On 2026-08-12, its official Akamai edge returned HTTP 403 for normal unattended requests, and no complete official unattended source was available. Re-enable it only when such a source is accessible and passes source-contract fixtures plus live completeness verification; do not bypass the edge protection.

The remaining allowlist is not enabled. Each company requires source-contract fixtures and live verification before it can be added safely. Re-check `SOURCES.md` before changing enabled policy.
