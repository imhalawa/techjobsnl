# Job Watch

Job Watch is a local terminal application for reviewing eligible Netherlands vacancies. It enables Mollie's, Airwallex's, and DataSnipper's Ashby boards, Adyen's, Databricks', and Reddit's Greenhouse boards, Booking.com's Jibe API, Funda's Recruitee board, bol.com's official careers API, ING's and ABN AMRO's official Netherlands careers sources, and eBay's official Netherlands careers pages.

## Install

GitHub Releases provide prebuilt binaries for macOS, Linux, and Windows on Intel/AMD and ARM64 computers. The Linux archives require glibc 2.35 or newer; the Windows archives require Windows 10 or newer.

### macOS and Linux

Install the latest release to `~/.local/bin`:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/imhalawa/job-tracker-nl/main/scripts/install.sh | sh
```

If `~/.local/bin` is not on `PATH`, the installer prints the command needed to add it. Rerun the installer to upgrade. Set `JOB_WATCH_VERSION=v0.1.0` to install a specific release or `JOB_WATCH_INSTALL_DIR` to choose another directory.

### Windows

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/imhalawa/job-tracker-nl/main/scripts/install.ps1 | iex
```

The installer places `job-watch.exe` under `%LOCALAPPDATA%\Programs\job-watch\bin` and adds that directory to the user `PATH`. Open a new terminal after the first installation. Rerun the installer to upgrade; set `$env:JOB_WATCH_VERSION = "v0.1.0"` first to install a specific release.

The initial binaries are not code-signed. macOS Gatekeeper or Windows SmartScreen may therefore ask for confirmation. Every release includes `SHA256SUMS`, and both installers verify the downloaded archive before installing it.

### Manual installation

Download the archive for your operating system and CPU from [GitHub Releases](https://github.com/imhalawa/job-tracker-nl/releases), verify it against `SHA256SUMS`, extract `job-watch` or `job-watch.exe`, and place it in a directory on `PATH`.

Rust users can build and install version `v0.1.0` directly from the tagged source:

```bash
cargo install --locked --git https://github.com/imhalawa/job-tracker-nl.git --tag v0.1.0 job-watch
```

### Uninstall

- macOS and Linux: remove `~/.local/bin/job-watch`, or the custom installation path.
- Windows: remove `%LOCALAPPDATA%\Programs\job-watch` and remove its `bin` directory from the user `PATH`.

Uninstalling the executable does not delete configuration or job history. Their locations are documented below.

## Run from source

From the repository root:

```bash
cargo run --release
```

On first start, the application creates `config.toml` from its built-in defaults. It uses the standard user configuration directory:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/job-watch/config.toml`
- macOS: `~/Library/Application Support/job-watch/config.toml`
- Windows: `%APPDATA%\job-watch\config.toml`

It loads stored active jobs at startup and does not contact a source until you press `r`.

## Publishing a release

Release tags must exactly match the version in `Cargo.toml`. After updating the version and passing `make check`, create and push an annotated tag:

```bash
git tag -a v0.1.0 -m "Job Watch v0.1.0"
git push origin v0.1.0
```

The release workflow builds six native archives: macOS, Linux, and Windows for x86-64 and ARM64. It publishes them with SHA-256 checksums and generated release notes only after all checks and builds pass. The Windows ARM64 GitHub runner is currently a public preview; the Rust Windows ARM64 target itself is fully supported.

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

The Analytics tab describes the observed postings; it does not claim to represent the whole labour market. Its Overview, Skills, Stacks, and Market sections combine tables with terminal charts. They show active demand, current-versus-previous posting momentum, role families, seniority, experience, work mode, employment, education, companies, common 2–5 skill stacks, personal learn-next signals, confidence, and exact evidence. The default window is 30 days. Use `t` for 7/30/90 days, `+`/`-` for a custom day count, `C`/`R`/`S`/`W` for shared filters, and `x` to clear filters.

Matching is local by default and uses the versioned software-industry bank in `assets/software-skills.json`. The bank keeps canonical hard and soft skills plus developer-community acronyms and aliases observed in real job postings. Unknown words are never promoted to skills. `analytics.minimum_skill_occurrence` filters one-off matches, `analytics.maximum_skills` limits each skill list, and `analytics.minimum_cooccurrence` controls the minimum shared-job count for stacks.

Set `analytics.provider` to `claude` or `codex` to optionally discover emerging terms with an installed, authenticated CLI. The app sends bounded job-description excerpts only when Analytics is opened, disables Claude tools or gives Codex an isolated empty working directory, validates strict JSON against exact posting text, and caches each attempt. Suggestions appear in Library → Skills for `a` approval or `d` rejection. They never change analytics automatically. Missing executables, invalid output, failure, and timeout safely fall back to the local bank. `analytics.ai_timeout_seconds` defaults to 60. No AI provider is required.

The Library stores starred jobs, skills, stacks, roles, and companies in SQLite. Skill status can be Known, Learning, or Interested; saved roles can be marked as targets. Closed and reopened starred jobs remain visible.

## Local data

The SQLite database is `.data/job-watch.sqlite3`, relative to the user configuration directory. It stores jobs, scan history, lifecycle changes, applied status, snapshots, versioned analytics facts, persistent Analytics filters, the Library, and reviewed AI suggestions. Unchanged descriptions reuse their cached extraction.

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
cargo test --test uber_test uber_live_returns_complete_unique_netherlands_jobs -- --ignored --exact --nocapture
cargo test --test successfactors_test flatexdegiro_live_returns_every_nl_job -- --ignored --exact --nocapture
cargo test --test pay_test -- --ignored --nocapture
```

The live smoke tests check every supported source, including flatexDEGIRO's current public SAP SuccessFactors board. The offline suite is the deterministic verification path.

## Company onboarding

The remaining allowlist is not enabled. Each company requires source-contract fixtures and live verification before it can be added safely. Re-check `SOURCES.md` before changing enabled policy.
