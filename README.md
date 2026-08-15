# TechJobsNL — Netherlands Tech Vacancies

[![CI](https://github.com/imhalawa/techjobsnl/actions/workflows/ci.yml/badge.svg)](https://github.com/imhalawa/techjobsnl/actions/workflows/ci.yml)
[![Release](https://github.com/imhalawa/techjobsnl/actions/workflows/release.yml/badge.svg)](https://github.com/imhalawa/techjobsnl/actions/workflows/release.yml)
[![Latest release](https://img.shields.io/github/v/release/imhalawa/techjobsnl?display_name=tag&sort=semver)](https://github.com/imhalawa/techjobsnl/releases/latest)
[![License: AGPL-3.0](https://img.shields.io/github/license/imhalawa/techjobsnl)](LICENSE)

**Research and track Netherlands tech vacancies from supported company career sources.**

TechJobsNL is an open-source, local-first Rust TUI with vacancy history and evidence-linked analytics.

![TechJobsNL: local job search and evidence-linked skill analytics](docs/images/hero.png)

> **Beta:** interfaces and configuration may change.

[Website](https://imhalawa.github.io/techjobsnl/) · [Install](#install) · [Workflow](#how-it-works) · [Analytics](#evidence-linked-analytics) · [Documentation](#documentation) · [Contributing](#development-and-contributing)

## What it does

- **Review vacancies:** search by company or title, inspect stored postings, and open official URLs.
- **Explore Analytics:** inspect the vacancies behind extracted skills and market facts.
- **Follow companies:** choose which supported sources future scans contact.
- **Track jobs:** save vacancies, mark applications, and retain closed or reopened history.
- **Keep data local:** configuration, history, analytics, and personal state stay on your machine.
- **Fail safely:** incomplete and failed company scans preserve the last trusted jobs instead of falsely closing them.

Coverage is limited to [supported companies](SUPPORTED_COMPANIES.md), not the full Netherlands labour market.

## Install

The latest release provides checksum-verified native archives for macOS, Linux, and Windows on x86-64 and ARM64.

### macOS and Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.sh | sh && "$HOME/.local/bin/techjobsnl"
```

The installer writes to `~/.local/bin` by default and tells you if that directory is not on `PATH`.

### Windows

```powershell
irm https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.ps1 | iex; if ($?) { & "$env:LOCALAPPDATA\Programs\techjobsnl\bin\techjobsnl.exe" }
```

The installer writes to `%LOCALAPPDATA%\Programs\techjobsnl\bin` and adds that directory to the user `PATH`. Open a new terminal if `techjobsnl` is not immediately available.

| Platform | Architectures | Requirement |
|---|---|---|
| macOS | Intel, Apple silicon | Unsigned beta binary |
| Linux | x86-64, ARM64 | glibc 2.35 or newer |
| Windows | x86-64, ARM64 | Windows 10 or newer; unsigned beta binary |

Because the beta binaries are not code-signed, macOS Gatekeeper or Windows SmartScreen may ask for confirmation. You can also download an archive and `SHA256SUMS` directly from [GitHub Releases](https://github.com/imhalawa/techjobsnl/releases/latest).

To install a specific release, set `TECHJOBSNL_VERSION=v0.1.0` on macOS/Linux or `$env:TECHJOBSNL_VERSION = "v0.1.0"` on Windows before running the installer. `TECHJOBSNL_INSTALL_DIR` changes the destination.

### Update or uninstall

Run the installer again to update the executable. Configuration, job history, and application state are preserved.

macOS and Linux:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/uninstall.sh | sh
```

Windows:

```powershell
irm https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/uninstall.ps1 | iex
```

The uninstallers remove the executable but preserve configuration and job history. The Windows uninstaller also removes its installation directory from the user `PATH`.

## How it works

1. Press `r` to scan the companies you follow.
2. Review **Active** or **New**, then use `/` to search by title or company.
3. Press `o` to open the official vacancy, `a` to mark an application, or `*` to save it.
4. Open **Analytics** and select a skill or market fact to inspect the matching vacancies.
5. Open **Settings → Companies** to focus later scans on the employers you care about.

![Active vacancies with the selected job's details](docs/images/jobs.png)

Press `?` for controls. Company choices save immediately and apply to later scans.

![Searchable company-following settings](docs/images/settings-companies.png)

## Evidence-linked analytics

Analytics covers skills, roles, seniority, experience, work mode, employment, education, companies, and recommendations from locally stored vacancies.

| Market overview | Hard-skill demand |
|---|---|
| [![Market overview with skill demand, role demand, recommendations, and matching vacancies](docs/images/analytics-overview.png)](docs/images/analytics-overview.png) | [![Hard-skill demand with the vacancies behind the selected result](docs/images/analytics-skills.png)](docs/images/analytics-skills.png) |

Results link to matching vacancy evidence and describe only your local dataset. **Stacks is visible but disabled while in development.**

Optional provider-assisted term discovery is review-gated; no AI provider is required.

## Local data and safe scans

Startup reads stored data and makes no source requests. A scan starts only when you press `r`.

- A **complete** company scan may add, update, close, or reopen its jobs.
- An **incomplete** or **failed** scan stores diagnostics without changing that company's jobs.
- One company failure does not discard another company's valid result.
- Company choices and title filters persist in `config.toml`; jobs and user state persist in SQLite.

The default data locations are:

| Platform | Configuration and database directory |
|---|---|
| Linux | `${XDG_CONFIG_HOME:-~/.config}/techjobsnl/` |
| macOS | `~/Library/Application Support/techjobsnl/` |
| Windows | `%APPDATA%\techjobsnl\` |

Vacancies can be rescanned, but deleting SQLite permanently removes application markers and Library choices. [Back up personal state before resetting](docs/DATA_AND_PRIVACY.md#backup).

## Build from source

You need Git, Make, and a Rust toolchain that supports Rust edition 2024.

```bash
git clone https://github.com/imhalawa/techjobsnl.git
cd techjobsnl
make run
```

`make run` requires an interactive terminal. You can also install the tagged source with Cargo:

```bash
cargo install --locked --git https://github.com/imhalawa/techjobsnl.git --tag v0.1.0 techjobsnl
```

## Documentation

| Guide | Use it for |
|---|---|
| [User guide](docs/USER_GUIDE.md) | Views, workflows, keyboard and mouse controls, Analytics, and Settings |
| [Configuration](docs/CONFIGURATION.md) | Paths, filters, themes, keys, company profiles, and source strategies |
| [Supported companies](SUPPORTED_COMPANIES.md) | Supported, disabled, and roadmap companies |
| [Source evidence](SOURCES.md) | Official endpoints, completeness evidence, live checks, and employer caveats |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Startup, scans, sources, browser, clipboard, Analytics, and reset |
| [Data and privacy](docs/DATA_AND_PRIVACY.md) | Stored data, network behavior, optional AI use, backup, and deletion |
| [Architecture](docs/ARCHITECTURE.md) | Runtime flow, module ownership, persistence, concurrency, and test boundaries |
| [Project structure](docs/PROJECT_STRUCTURE.md) | Repository map and where changes belong |
| [Contributing](CONTRIBUTING.md) | Development workflow, checks, source safety, and pull requests |

## Development and contributing

```bash
make check      # formatting, Clippy, release checks, and offline tests
make test-live  # ignored tests that contact external career sites
```

Live tests are separate because external sources change. Read [Contributing](CONTRIBUTING.md) before changing an adapter.

## Scope and independence

TechJobsNL is an independent open-source project. It is not affiliated with, endorsed by, or sponsored by any listed company. Company names and trademarks belong to their owners, and job information can change at any time.

A company brand does not prove the legal employer or current visa-sponsor status. Confirm the employment entity and application requirements on the official vacancy. [Source evidence](SOURCES.md) keeps verified facts, inferences, and caveats separate.

## License

Copyright © 2026 Mohamed Halawa.

The source code is licensed under the [GNU Affero General Public License v3.0 only](LICENSE). The project does not claim ownership of third-party company names, trademarks, logos, or job-posting content. The software is provided without warranty under the license terms.

## Support

Support the open-source project:

[![Buy Me a Coffee](https://img.shields.io/badge/Buy_Me_a_Coffee-Support-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=000000)](https://buymeacoffee.com/imhalawa)
