# TechJobsNL

[![CI](https://github.com/imhalawa/techjobsnl/actions/workflows/ci.yml/badge.svg)](https://github.com/imhalawa/techjobsnl/actions/workflows/ci.yml)
[![Release](https://github.com/imhalawa/techjobsnl/actions/workflows/release.yml/badge.svg)](https://github.com/imhalawa/techjobsnl/actions/workflows/release.yml)

**Find Netherlands tech jobs by the skills they mention—not only by job title.**

TechJobsNL is a local terminal job finder built with Rust and Ratatui. It collects vacancies from verified official company career sources, keeps their lifecycle in SQLite, and connects every analytics result back to the job postings behind it.

![TechJobsNL hero](docs/images/hero.png)

> **Beta v0.1.0:** the core workflow is tested, but the interface and configuration may still change. Back up your configuration and database before upgrading.

## Why TechJobsNL

- **Follow the companies you care about:** search a 66-company catalog in Settings and enable or disable sources without editing TOML. The shipped beta follows 65 companies.
- **Review jobs, not scraped fragments:** open the official posting, read the stored description, mark applications, and keep closed or reopened roles in history.
- **Find jobs through skills:** select a hard or soft skill in Analytics to see vacancies whose stored descriptions contain matching evidence.
- **Trust partial failures:** only complete company scans update lifecycle state. Incomplete and failed scans preserve the last trusted jobs.
- **Keep data local:** jobs, history, application state, analytics facts, settings, and the personal library stay in a local SQLite database.
- **Use keyboard or mouse:** the interface supports responsive layouts, search, scrolling, clickable rows and tabs, and a draggable job/details divider.

The shipped catalog contains **66 company profiles across 35 source strategies**. Coverage is not the whole Netherlands labour market; it is the verified set documented in [Supported companies](SUPPORTED_COMPANIES.md) and [Source evidence](SOURCES.md).

## Quick start from source

You need Git, Make, and a Rust toolchain that supports Rust edition 2024.

```bash
git clone https://github.com/imhalawa/techjobsnl.git
cd techjobsnl
make run
```

`make run` needs an interactive terminal. Startup loads stored jobs and makes no source requests. Press `r` when you want to scan followed companies.

On first start, TechJobsNL creates its configuration and database under:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/techjobsnl/`
- macOS: `~/Library/Application Support/techjobsnl/`
- Windows: `%APPDATA%\techjobsnl\`

An older `job-watch/config.toml` is reused automatically when the new path does not exist.

## Install a tagged release

Tagged releases publish six native archives for macOS, Linux, and Windows on x86-64 and ARM64. Linux binaries require glibc 2.35 or newer; Windows binaries require Windows 10 or newer. The beta binaries are not code-signed, so macOS Gatekeeper or Windows SmartScreen may ask for confirmation.

These installers require a published entry on [GitHub Releases](https://github.com/imhalawa/techjobsnl/releases). If no tag is published yet, use the source quick start above.

### macOS and Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.sh | sh
```

The installer verifies the release checksum and writes to `~/.local/bin` by default. Use `TECHJOBSNL_VERSION=v0.1.0` for a specific tag or `TECHJOBSNL_INSTALL_DIR` for another directory.

### Windows

```powershell
irm https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.ps1 | iex
```

The installer verifies the release checksum, writes to `%LOCALAPPDATA%\Programs\techjobsnl\bin`, and adds that directory to the user `PATH`. Open a new terminal after installation. Set `$env:TECHJOBSNL_VERSION = "v0.1.0"` before running the installer to select a specific tag.

You can also download an archive and `SHA256SUMS` from [GitHub Releases](https://github.com/imhalawa/techjobsnl/releases), or install a tag with Cargo:

```bash
cargo install --locked --git https://github.com/imhalawa/techjobsnl.git --tag v0.1.0 techjobsnl
```

Uninstalling the executable does not delete configuration or job history.

## Main workflow

1. Press `r` to scan followed companies.
2. Review **Active** or **New**, use `/` to search by title or company, and press `o` to open the official posting.
3. Press `a` to mark an application or `*` to save a job.
4. Open **Analytics** to explore skills and market facts, then open the exact matching vacancies shown as evidence.
5. Open **Settings → Companies** to search the catalog and follow or unfollow companies.

![TechJobsNL active jobs with selected-job details](docs/images/jobs.png)

Press `?` in the app for the controls available in the current view.

### Choose companies

Changes save immediately, hide unfollowed-company jobs, and affect later scans without starting one.

![Company following settings with industry and scale](docs/images/settings-companies.png)

## Analytics

Analytics describes only locally stored, eligible postings. It covers hard and soft skills, roles, seniority, experience, work mode, employment, education, companies, and learn-next recommendations. Compact top-10 charts show the leading skill and role demand. **Stacks is visible but disabled while it remains work in progress.**

| Market overview | Hard-skill demand |
|---|---|
| [![Analytics overview with skill demand, role demand, recommendations, and matching jobs](docs/images/analytics-overview.png)](docs/images/analytics-overview.png) | [![Hard-skill demand with matching vacancies](docs/images/analytics-skills.png)](docs/images/analytics-skills.png) |

Select a skill or market fact to inspect the vacancies behind it. Counts come from the locally observed postings and are not presented as the whole Netherlands market.

Local matching uses the versioned bank in `assets/software-skills.json`; unknown words are not promoted automatically. Optional Claude or Codex CLI discovery can suggest emerging terms, but strict validation and explicit approval are required before a suggestion affects later extraction. No AI provider is required.

## Safety and privacy

- Startup does not scan automatically.
- Complete scans may add, update, close, or reopen a company's jobs.
- Incomplete or failed scans record diagnostics without changing that company's jobs.
- One company failure does not discard another company's valid result.
- Company choices and title filters persist in `config.toml`; jobs and user state persist in SQLite.
- Deleting the database permanently removes jobs, history, application markers, diagnostics, analytics state, and library choices.

A company brand does not prove the legal employer or current visa-sponsor status. Always confirm the employment entity and application requirements on the official vacancy. See [Data and privacy](docs/DATA_AND_PRIVACY.md) and [Source evidence](SOURCES.md).

## Documentation

| Guide | Use it for |
|---|---|
| [User guide](docs/USER_GUIDE.md) | Views, workflows, keyboard and mouse controls, Analytics, and Settings |
| [Configuration](docs/CONFIGURATION.md) | Paths, filters, themes, keys, company profiles, and all 35 source strategies |
| [Supported companies](SUPPORTED_COMPANIES.md) | The 65 enabled companies, disabled source, and roadmap |
| [Source evidence](SOURCES.md) | Official endpoints, completeness evidence, live checks, and employer caveats |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Startup, scans, sources, browser, clipboard, Analytics, and reset |
| [Data and privacy](docs/DATA_AND_PRIVACY.md) | Stored data, network behavior, optional AI use, backup, and deletion |
| [Architecture](docs/ARCHITECTURE.md) | Runtime flow, module ownership, persistence, concurrency, and test boundaries |
| [Project structure](docs/PROJECT_STRUCTURE.md) | Repository map and where changes belong |
| [Contributing](CONTRIBUTING.md) | Development workflow, checks, source safety, and pull requests |
| [Resume entry](docs/RESUME.md) | Accurate project wording and interview topics |

## Development

```bash
make check      # formatting, Clippy, release checks, and offline tests
make test-live  # ignored tests that contact external career sites
```

GitHub Actions runs formatting, Clippy, offline tests, and a release build on Linux, macOS, and Windows for every pull request and push to `main`. Version tags matching `v*` run the six-target release workflow and publish checksum-verified archives for all three operating systems.

Live tests are separate because external availability, counts, rate limits, and page contracts can change without a code change. Read [Contributing](CONTRIBUTING.md) before changing a source adapter.

## Purpose and independence

TechJobsNL is an independent, open-source learning project. It is not affiliated with, endorsed by, or sponsored by any listed company. Company names and trademarks belong to their owners, and job information can change at any time.

## License

Copyright © 2026 Mohamed Halawa.

The source code is licensed under the [GNU Affero General Public License v3.0 only](LICENSE). The project does not claim ownership of third-party company names, trademarks, logos, or job-posting content. The software is provided without warranty under the license terms.
