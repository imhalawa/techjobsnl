# TechJobsNL

TechJobsNL is a local Rust terminal application for finding, reviewing, and analysing Netherlands job vacancies. It collects jobs from official company career sources, applies configurable country and title filters, tracks vacancy history in SQLite, and keeps job-market analytics explainable with exact posting evidence.

![TechJobsNL active jobs](docs/images/jobs.png)

## Purpose and independence

TechJobsNL is an independent, open-source learning project. It organizes publicly accessible job postings from official company career pages so its author and users can learn about Rust, terminal interfaces, data collection, and job-market analysis.

This project is not affiliated with, endorsed by, or sponsored by any company listed in the application. Company names and trademarks belong to their respective owners. Job information can change at any time; always verify role details and application requirements on the official posting.

## Features

- **One review queue:** active, new, applied, closed, and reopened jobs with full descriptions and official links.
- **Verified source lifecycle:** complete scans may update and close jobs; incomplete or failed scans preserve the last trusted state.
- **Search and filters:** search by title or company and filter by company, new, or applied status.
- **Local analytics:** skills, technology stacks, roles, seniority, experience, work mode, employment, education, companies, momentum, confidence, and posting evidence.
- **Personal library:** save multiple jobs, skills, stacks, roles, and companies; saved job rows show a star, actions show footer feedback, and long-running work shows an animated loader.
- **Local persistence:** SQLite stores jobs, snapshots, scan history, applied state, analytics facts, filters, and library choices.
- **Keyboard and mouse:** responsive terminal layouts, configurable action keys, clickable tabs and rows, scrolling, and a draggable job/details divider.
- **Broad source coverage:** the shipped catalog contains 60+ company profiles across 30+ configured official-source strategies. See [source evidence](SOURCES.md) for the current companies and caveats.

## Install

GitHub Releases provide prebuilt binaries for macOS, Linux, and Windows on Intel/AMD and ARM64 computers. The Linux archives require glibc 2.35 or newer; the Windows archives require Windows 10 or newer.

### macOS and Linux

Install the latest release to `~/.local/bin`:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.sh | sh
```

If `~/.local/bin` is not on `PATH`, the installer prints the command needed to add it. Rerun it to upgrade. Set `TECHJOBSNL_VERSION=v0.1.0` to install a specific release or `TECHJOBSNL_INSTALL_DIR` to choose another directory.

### Windows

Open PowerShell and run:

```powershell
irm https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.ps1 | iex
```

The installer places `techjobsnl.exe` under `%LOCALAPPDATA%\Programs\techjobsnl\bin` and adds that directory to the user `PATH`. Open a new terminal after the first installation. Rerun it to upgrade; set `$env:TECHJOBSNL_VERSION = "v0.1.0"` first to install a specific release.

The initial binaries are not code-signed. macOS Gatekeeper or Windows SmartScreen may ask for confirmation. Every release includes `SHA256SUMS`, and both installers verify the downloaded archive.

### Manual or Cargo installation

Download the archive for your operating system and CPU from [GitHub Releases](https://github.com/imhalawa/techjobsnl/releases), verify it against `SHA256SUMS`, extract `techjobsnl` or `techjobsnl.exe`, and place it on `PATH`.

Rust users can install a tagged version from source:

```bash
cargo install --locked --git https://github.com/imhalawa/techjobsnl.git --tag v0.1.0 techjobsnl
```

### Uninstall

- macOS and Linux: remove `~/.local/bin/techjobsnl`, or the custom installation path.
- Windows: remove `%LOCALAPPDATA%\Programs\techjobsnl` and remove its `bin` directory from the user `PATH`.

Uninstalling the executable does not delete configuration or job history.

## Run from source

You need a Rust toolchain with Rust 2024 edition support.

```bash
cargo run --release
```

The first run creates the user configuration and database. It loads saved jobs without making a network request; press `r` when you want to scan enabled sources.

Configuration paths:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/techjobsnl/config.toml`
- macOS: `~/Library/Application Support/techjobsnl/config.toml`
- Windows: `%APPDATA%\techjobsnl\config.toml`

Upgrades reuse an existing `job-watch/config.toml` and its configured database automatically. New installations use the paths above.

Press `?` inside the app for the complete context-aware key guide.

## Documentation

| Guide | Purpose |
|---|---|
| [User guide](docs/USER_GUIDE.md) | Every view, workflow, keyboard action, and mouse action |
| [Configuration](docs/CONFIGURATION.md) | All settings, defaults, validation rules, and source strategy fields |
| [Architecture](docs/ARCHITECTURE.md) | Runtime flow, modules, persistence, scan safety, and analytics design |
| [Data and privacy](docs/DATA_AND_PRIVACY.md) | Network access, local storage, optional AI use, deletion, and recovery |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common startup, scan, terminal, browser, clipboard, and AI issues |
| [Resume entry](docs/RESUME.md) | Accurate project wording, bullets, and interview talking points |
| [Source evidence](SOURCES.md) | Official endpoints, completeness evidence, sponsor caveats, and live checks |

## Verification

Run the deterministic offline suite:

```bash
make test
```

Run formatting, Clippy, Makefile checks, and all offline tests:

```bash
make check
```

Live source tests are ignored by default because they contact external services:

```bash
make test-live
```

## Publish a release

Release tags must exactly match the version in `Cargo.toml`. After updating the version and passing `make check`, create and push an annotated tag:

```bash
git tag -a v0.1.0 -m "TechJobsNL v0.1.0"
git push origin v0.1.0
```

The release workflow validates the tag, runs offline checks, builds six native archives for macOS, Linux, and Windows on x86-64 and ARM64, generates SHA-256 checksums, and publishes generated release notes. The Windows ARM64 GitHub runner is currently a public preview.

Live vacancy counts change and a company brand does not prove the legal employer or visa-sponsor status. Read [SOURCES.md](SOURCES.md) and confirm the employment entity before relying on sponsor information.

## License

Copyright © 2026 Mohamed Halawa.

The project source code is licensed under the [GNU Affero General Public License v3.0 only](LICENSE). The project does not claim ownership of third-party company names, trademarks, logos, or job-posting content, and the project license does not grant rights to them. The software is provided without warranty under the terms of the license.

Job eligibility is controlled by `[filters]` in `config.toml`. The shipped country and title patterns preserve the current Netherlands engineering defaults. An empty `include_title_patterns` list allows every title; an empty `exclude_title_patterns` list excludes none.

## Analytics

The Analytics tab describes the observed postings; it does not claim to represent the whole labour market. Its Overview, Skills, Stacks, and Market sections combine tables with terminal charts. They show active demand, current-versus-previous posting momentum, role families, seniority, experience, work mode, employment, education, companies, common 2–5 skill stacks, personal learn-next signals, confidence, and exact evidence. The default window is 30 days. Use `t` for 7/30/90 days, `+`/`-` for a custom day count, `C`/`R`/`S`/`W` for shared filters, and `x` to clear filters.

Matching is local by default and uses the versioned software-industry bank in `assets/software-skills.json`. The bank keeps canonical hard and soft skills plus developer-community acronyms and aliases observed in real job postings. Unknown words are never promoted to skills. `analytics.minimum_skill_occurrence` filters one-off matches, `analytics.maximum_skills` limits each skill list, and `analytics.minimum_cooccurrence` controls the minimum shared-job count for stacks.

Set `analytics.provider` to `claude` or `codex` to optionally discover emerging terms with an installed, authenticated CLI. The app sends bounded job-description excerpts only when Analytics is opened, disables Claude tools or gives Codex an isolated empty working directory, validates strict JSON against exact posting text, and caches each attempt. Suggestions appear in Library → Skills for `a` approval or `d` rejection. They never change analytics automatically. Missing executables, invalid output, failure, and timeout safely fall back to the local bank. `analytics.ai_timeout_seconds` defaults to 60. No AI provider is required.

The Library stores starred jobs, skills, stacks, roles, and companies in SQLite. Skill status can be Known, Learning, or Interested; saved roles can be marked as targets. Closed and reopened starred jobs remain visible.

## Local data

The SQLite database is `.data/techjobsnl.sqlite3`, relative to the user configuration directory. It stores jobs, scan history, lifecycle changes, applied status, snapshots, versioned analytics facts, persistent Analytics filters, the Library, and reviewed AI suggestions. Unchanged descriptions reuse their cached extraction.

It is safe to delete the database while TechJobsNL is not running. Deletion permanently removes all local history and applied markers; the next run creates an empty database, and the next complete scan treats every eligible job as new.

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
cargo test --test getnoticed_test getnoticed_live_returns_complete_unique_brand_new_day_jobs -- --ignored --exact --nocapture
cargo test --test ebay_test ebay_live_returns_complete_unique_netherlands_jobs -- --ignored --nocapture
cargo test --test uber_test uber_live_returns_complete_unique_netherlands_jobs -- --ignored --exact --nocapture
cargo test --test successfactors_test flatexdegiro_live_returns_every_nl_job -- --ignored --exact --nocapture
cargo test --test pay_test -- --ignored --nocapture
```

The live smoke tests check every supported source, including flatexDEGIRO's current public SAP SuccessFactors board. The offline suite is the deterministic verification path.

## Company onboarding

The remaining allowlist is not enabled. Each company requires source-contract fixtures and live verification before it can be added safely. Re-check `SOURCES.md` before changing enabled policy.
