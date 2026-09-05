# Configuration

TechJobsNL uses one TOML file. Shipped defaults are in [config.toml](../config.toml).

## File location and updates

- Linux: `${XDG_CONFIG_HOME:-~/.config}/techjobsnl/config.toml`
- macOS: `~/Library/Application Support/techjobsnl/config.toml`
- Windows: `%APPDATA%\techjobsnl\config.toml`

On first start, the app creates the file. Later starts merge newly shipped company profiles while preserving each existing company's `enabled` choice and unrelated user configuration. Invalid configuration stops startup and reports the absolute file path and failing field.

`database_path` is resolved relative to the directory containing the user configuration file.

## Top-level fields

| Field | Shipped value | Meaning |
|---|---:|---|
| `schema_version` | `1` | Configuration schema; it must be exactly `1` |
| `database_path` | `.data/techjobsnl.sqlite3` | SQLite path, relative to the user configuration directory unless absolute |

## Scan settings

```toml
[scan]
concurrency = 4
timeout_seconds = 20
retry_count = 2
user_agent = "techjobsnl/0.1 (+local personal job research)"
```

| Field | Meaning |
|---|---|
| `concurrency` | Maximum companies scanned at the same time |
| `timeout_seconds` | Timeout for each source attempt |
| `retry_count` | Retry budget for retryable timeouts, rate limits, and server errors |
| `user_agent` | HTTP User-Agent sent to job sources |

Non-retryable configuration, authentication, schema, and ordinary client errors fail directly. A source-provided `Retry-After` value overrides fallback retry delay.

## Eligibility filters

```toml
[filters]
countries = ["NL"]
new_job_max_age_days = 7
include_title_patterns = ["software engineer", "backend developer"]
exclude_title_patterns = ["manager", "director"]
```

- `countries` must contain uppercase two-letter country codes.
- `new_job_max_age_days` must be greater than zero.
- Title patterns are case-insensitive regular expressions and must compile successfully.
- Empty `include_title_patterns` accepts every title.
- Empty `exclude_title_patterns` excludes no title.
- A job must resolve to an allowed country and satisfy title rules to be eligible.
- An unresolved location makes the company result incomplete rather than silently accepting or rejecting uncertain jobs.

## Analytics settings

Analytics settings control the local facts used to explore observed market patterns and find vacancies matching a selected skill.

```toml
[analytics]
provider = "local"
minimum_skill_occurrence = 2
maximum_skills = 50
ai_timeout_seconds = 60
minimum_cooccurrence = 3
```

| Field | Allowed values or rule | Meaning |
|---|---|---|
| `provider` | `local`, `claude`, `codex` | Local extraction, with optional CLI-assisted emerging-term discovery |
| `minimum_skill_occurrence` | Integer greater than zero | Hides one-off skills and recommendations below the threshold |
| `maximum_skills` | Integer greater than zero | Maximum rows in skill, stack, and recommendation lists |
| `ai_timeout_seconds` | Integer greater than zero | Timeout for an optional provider CLI call |
| `minimum_cooccurrence` | Integer greater than zero | Minimum jobs shared by a reported technology stack |

The local provider is complete on its own. Optional providers only suggest emerging terms for later human review; they do not replace local analytics or auto-approve terms.

## UI settings

```toml
[ui]
theme = "clean-dark"
unicode_icons = true

[ui.theme_overrides]
focused_border = "cyan"
error = "#ff5555"
```

The only valid built-in themes are `clean-dark` and `clean-light`.

Optional override keys:

- `background`
- `focused_border`
- `unfocused_border`
- `selected_row`
- `primary_text`
- `muted_text`
- `open`
- `new`
- `applied`
- `warning`
- `error`

Colours must be a supported named ANSI colour or `#RRGGBB`. Set `unicode_icons = false` for ASCII status symbols.

## Keybindings

```toml
[keybindings]
scan = "r"
search = "/"
filter = "f"
toggle_applied = "a"
history = "h"
open = "o"
copy = "c"
help = "?"
quit = "q"
```

Each action binding must be one non-control character, must be unique, and cannot be `j`, `k`, `J`, or `K` because those are fixed navigation keys. Tab, Esc, Enter, Space, arrows, Analytics controls, Library controls, and mouse behavior are fixed.

## Company profiles

```toml
[[companies]]
id = "mollie"
name = "Mollie"
industry = "Fintech, Payments, Financial services"
scale = "1,000–1,999 · Large company"
enabled = true

[companies.location_country_overrides]
"Amsterdam office" = "NL"

[companies.source]
strategy = "ashby"
board = "mollie"
```

| Field | Rule |
|---|---|
| `id` | Stable internal company identity |
| `name` | Display name |
| `industry` | Display metadata used by source details |
| `scale` | Display metadata; treat it as researched context, not an audited live headcount |
| `enabled` | Only enabled companies are scheduled for scans |
| `location_country_overrides` | Optional exact source-label to country-code mapping for otherwise unresolved locations |
| `source` | Adapter strategy and its required official-source fields |

Settings → Companies can change `enabled` without editing TOML. The choice is written immediately, the current job and Analytics views reload without the company, and future scans skip it. No scan starts automatically. Disabling a company does not delete its stored history or applied state, and enabling no companies is valid.

## Runtime-supported source strategies

The shipped catalog currently uses all **35** strategies below. Optional fields are marked with `?`.

| Strategy | Required source fields |
|---|---|
| `ashby` | `board` |
| `greenhouse` | `board`, `country_filter?` |
| `jibe` | `base_url`, `client` |
| `ebay` | `listing_url` |
| `recruitee` | `base_url` |
| `personio` | `base_url` |
| `lever` | `api_url`, `country_filter?` |
| `workable` | `account`, `country_filter?` |
| `workday` | `base_url`, `tenant`, `site`, `country`, `country_code` |
| `yuki` | `feed_url` |
| `teamtailor` | `feed_url`, `employer` |
| `bol` | `base_url` |
| `coolblue` | `listing_url` |
| `pay` | `listing_url` |
| `buckaroo` | `listing_url` |
| `rabobank` | `base_url`, `country` |
| `eneco` | `listing_url` |
| `exact` | `listing_url` |
| `afas` | `listing_url` |
| `ns` | `listing_url` |
| `achmea` | `listing_url` |
| `chipsoft` | `listing_url` |
| `anwb` | `feed_url` |
| `postnl` | `api_url` |
| `pggm` | `listing_url` |
| `amazon` | `search_url` |
| `uber` | `api_url` |
| `microsoft` | `search_url` |
| `deel` | `board_url` |
| `successfactors` | `listing_url`, `employer` |
| `google` | `search_url` |
| `successfactors-api` | `base_url` |
| `albert-heijn` | `base_url` |
| `ing` | `listing_url` |
| `getnoticed` | `base_url`, `country_filter?` |

URLs must be HTTPS. Several employer-specific adapters also validate exact official hosts, paths, identifiers, or employer names; a structurally valid but untrusted endpoint is rejected. `paged-html` and `unsupported` exist as configuration model variants but are not wired as enabled runtime sources, so do not use them for an enabled company.

For current company mappings, official endpoints, completeness rules, and legal-employer caveats, read [SOURCES.md](../SOURCES.md).
