# Job Watch

Job Watch is a local terminal application for reviewing eligible Netherlands vacancies. It enables Mollie's, Airwallex's, and DataSnipper's Ashby boards, Adyen's, Databricks', and Reddit's Greenhouse boards, Booking.com's Jibe API, Funda's Recruitee board, bol.com's official careers API, ING's and ABN AMRO's official Netherlands careers sources, and eBay's official Netherlands careers pages.

## Run

From the repository root:

```bash
cargo run --release
```

On first start, the application creates `config.toml` from its built-in defaults. It uses the standard user configuration directory:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/job-watch/config.toml`
- macOS: `~/Library/Application Support/job-watch/config.toml`
- Windows: `%APPDATA%\job-watch\config.toml`

It loads stored active jobs at startup and does not contact a source until you press `r`.

## Keys

- `r`: scan enabled sources
- `Tab` or `Esc`: focus the navigation tabs; arrow keys or `j`/`k` select a tab; `Enter` opens it
- Inside a tab, arrow keys or `j`/`k`: move through that tab's items
- `J`/`K`: scroll job details
- `/`: search by job title or company; `Enter` accepts the search, while `Esc` cancels and clears it during editing (press `/`, then `Esc`, to clear an accepted search)
- `f`: cycle company, new, and applied filters, then clear the filter
- `h`: switch between active jobs and history
- `a`: mark or unmark the selected job as applied
- `o`: open the selected job in the default browser
- `c`: copy the selected job URL to the system clipboard
- `?`: show or hide help
- `q`: quit
- Mouse: click tabs and rows, scroll lists or details, and drag the divider between job list and details to resize both panes for the current session

The Settings tab edits the user `config.toml`. It controls the new-job age, countries, included title patterns, and excluded title patterns. Clear included titles to allow every job type, including non-engineering roles. Country and title changes apply on the next scan; jobs without a publication date are not considered new.

Job eligibility is controlled by `[filters]` in `config.toml`. The shipped country and title patterns preserve the current Netherlands engineering defaults. An empty `include_title_patterns` list allows every title; an empty `exclude_title_patterns` list excludes none.

## Analytics

The Analytics tab describes the currently observed jobs; it does not claim to represent the whole labour market. It shows native horizontal charts for configured skills and related-skill overlap, exact counts and denominators, extraction coverage, source health, remote mode, seniority, experience, education, employment type, and evidence from matching descriptions. It runs locally without an AI service or network request.

Edit `[analytics.skills]` in the user `config.toml` to add, remove, or rename canonical skill labels and their literal aliases. For example, `SQL = ["sql", "postgres", "postgresql"]`. Short aliases use word boundaries, so `Go` does not match `ongoing`. `analytics.minimum_cooccurrence` controls the minimum shared-job count for related skills; its default is 10.

## Local data

The SQLite database is `.data/job-watch.sqlite3`, relative to the user configuration directory. It stores jobs, scan history, lifecycle changes, applied status, snapshots, and versioned analytics facts. Unchanged descriptions reuse their cached extraction.

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
cargo test --test hosted_ats_test greenhouse_live_returns_complete_unique_jobs -- --ignored --nocapture
cargo test --test hosted_ats_test databricks_live_returns_complete_unique_netherlands_jobs -- --ignored --exact --nocapture
cargo test --test hosted_ats_test reddit_live_returns_complete_unique_netherlands_jobs -- --ignored --exact --nocapture
cargo test --test hosted_ats_test jibe_live_returns_complete_unique_jobs -- --ignored --nocapture
cargo test --test hosted_ats_test recruitee_live_returns_complete_unique_jobs -- --ignored --nocapture
cargo test --test structured_sources_test bol_live_returns_complete_unique_jobs_and_working_urls -- --ignored --exact --nocapture
cargo test --test html_sources_test ing_live_returns_complete_unique_jobs -- --ignored --exact --nocapture
cargo test --test getnoticed_test getnoticed_live_returns_complete_unique_abn_jobs -- --ignored --exact --nocapture
cargo test --test ebay_test ebay_live_returns_complete_unique_netherlands_jobs -- --ignored --nocapture
```

The live smoke tests check Mollie's, Airwallex's, DataSnipper's, Adyen's, Databricks', Reddit's, Booking.com's, Funda's, bol.com's, ING's, ABN AMRO's, and eBay's current public payloads. The offline suite is the deterministic verification path.

## Company onboarding

The remaining allowlist is not enabled. Each company requires source-contract fixtures and live verification before it can be added safely. Re-check `SOURCES.md` before changing enabled policy.
