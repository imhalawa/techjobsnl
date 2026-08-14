# Realistic job-description analytics for `job-watch`

## Recommendation

Build **descriptive analytics for the jobs this app actually tracks**, not claims about the whole Dutch labour market. Online job ads have sector, occupation, and geographic coverage bias, so the UI should say “observed postings” rather than “market demand” or “job shortage” ([Cedefop representativeness study](https://www.cedefop.europa.eu/en/publications/6217)).

The best path is:

1. Use **ESCO v1.2.1** as the canonical taxonomy [organized classification] because this product tracks European jobs. ESCO provides stable concept URIs, preferred terms, alternative terms, a hierarchy, and English and Dutch labels ([ESCO overview](https://esco.ec.europa.eu/en/about-esco/what-esco), [download structure](https://esco.ec.europa.eu/en/structure-esco-downloadable-datasets)).
2. Extract structured fields first, then use text rules only as fallback. Schema.org already defines job fields for skills, experience, education, employment type, remote work, and applicant location ([Schema.org `JobPosting`](https://schema.org/JobPosting)).
3. Count each skill **once per unique posting**, retain the matching text as evidence, and show every denominator and coverage count.
4. Add native Ratatui charts: horizontal bars for rankings, line/area charts for trends, and sparklines for compact summaries. Defer custom `Canvas` graphics.
5. Keep the first version local, deterministic [same input gives the same result], and explainable. Add an NLP model only after a manually reviewed sample proves that dictionary matching misses enough useful skills to justify it.

## What the current app and cache support

The current model already stores title, department, team, employment type, locations, countries, full description, raw source payload, publication time, first/last-seen times, closed/reopened/applied times, and a stable `(company_id, source_id)` key ([job model](src/domain/job.rs), [database schema](src/storage/schema.rs)). Changed postings are retained as content-addressed snapshots ([storage implementation](src/storage/store.rs)).

Current analytics is simpler: it searches configured literal aliases inside descriptions, counts one hit per active job, and applies the current company/search filters ([analytics implementation](src/ui/app.rs), [current rendering](src/ui/render.rs), [default aliases](config.toml)). This is explainable, but it has no taxonomy, evidence span, ambiguity handling, extraction coverage, historical cohort, or normalized facts beyond skills.

### Read-only cache audit on 2026-08-14

| Measure | Result | Meaning |
|---|---:|---|
| Jobs | 1,200 | All were active, had a non-empty description, and had `published_at`. |
| Companies | 9 | All 9 enabled sources were healthy and their scans completed. |
| First-seen range | 2026-08-13 23:27:23–23:27:34 UTC | The cache currently contains one discovery window of about 11 seconds. |
| Published range | 2023-07-03–2026-08-14 | This is the publication range of jobs still open now, not a historical sample of all jobs published in that period. |
| Snapshots | 2,324 across 1,200 jobs | 1,124 jobs gained a second content hash during two scans about 57 minutes apart. The stored pairs and source diff confirm a one-time parser migration from plain text to Markdown-preserving descriptions; titles and URLs stayed stable. These are representation changes, not evidence of employer edits. |
| Missing department / team / employment type | 59 / 458 / 286 | These fields need an explicit `Unknown` bucket; silently removing them would bias percentages. |
| Duplicate groups under current `content_hash` | 0 | This does not rule out cross-source duplicates because the current hash includes URLs and other source-specific fields. |

The audit used the installed user database and these read-only queries:

```sql
SELECT count(*), sum(source_open), sum(description <> ''),
       sum(published_at IS NOT NULL)
FROM jobs;

SELECT min(published_at), max(published_at),
       min(first_seen_at), max(first_seen_at)
FROM jobs;

SELECT count(*), count(DISTINCT company_id || char(0) || source_id)
FROM job_snapshots;
```

**Consequence:** current skill, company, location, remote, experience, and seniority snapshots can be useful now. A genuine weekly trend needs future scans so that new, changed, and closed postings are observed over time. Change analytics must distinguish parser/extractor version changes from source-content changes; the 1,124 extra hashes came from the Markdown parser migration.

## Skill extraction and normalization

### Canonical taxonomy

ESCO is the right primary taxonomy for this app. Its current skills pillar has **13,939 concepts** with preferred terms, non-preferred terms, descriptions, hierarchy, and relationships; it covers 28 languages, including English and Dutch ([ESCO skills pillar](https://esco.ec.europa.eu/en/classification/skill)). Non-preferred terms include synonyms, spelling variants, and abbreviations, while hidden terms exist specifically for search and text mining ([non-preferred terms](https://esco.ec.europa.eu/en/about-esco/escopedia/escopedia/non-preferred-term), [hidden terms](https://esco.ec.europa.eu/en/about-esco/escopedia/escopedia/hidden-term)).

Use the **ESCO URI** as the stored identity and the preferred term as the display label. This makes `postgres`, `PostgreSQL`, and any reviewed local alias one concept instead of separate bars. Keep the ESCO version beside extracted results because the classification changes over time; ESCO publishes downloadable CSV/ODS packages and an API ([ESCO downloads](https://esco.ec.europa.eu/en/use-esco/download), [ESCO API](https://esco.ec.europa.eu/en/use-esco/use-esco-services-api/esco-web-service-api)).

O*NET is useful as optional enrichment for software tools, but not as the main taxonomy: its occupation taxonomy covers work in the **U.S. economy**, while its downloadable database includes software-skill examples and quarterly updates ([O*NET database](https://www.onetcenter.org/database.html)). For Dutch and wider European jobs, mixing O*NET and ESCO in the MVP adds mapping work without fixing a demonstrated gap.

### Deterministic MVP extractor

For every new `content_hash`:

1. Normalize Unicode case and whitespace while preserving original character offsets.
2. Match ESCO preferred/non-preferred terms plus user aliases, longest phrase first.
3. Require term boundaries and maintain context rules for ambiguous short names such as `Go`, `R`, and `C`.
4. Count a canonical skill once per posting, even when it appears many times.
5. Store the matched text, nearby context, matcher version, taxonomy version, and extraction method.
6. Let the detail pane open the matching job and show the evidence text.

This is a deliberately high-precision [few false matches] first version. A full taxonomy matcher can still miss new or implicit skills: SkillSpan was created because predefined inventories and available labelled data were limited; its 14,500 sentences and more than 12,500 expert-annotated spans also show that proper extraction needs span-level evaluation ([SkillSpan paper](https://aclanthology.org/2022.naacl-main.366/)).

### Validation before calling it realistic

Create one manually reviewed gold set of **100 descriptions**, spread across companies and job families. The number 100 is a practical product starting point, not a statistical guarantee. Label exact skill spans and the structured facts below, then report precision, recall, and F1 by fact type; keep the examples that fail as regression tests.

Do not expose a model-style “confidence score” unless it is calibrated against reviewed data. Instead, expose understandable provenance [where the result came from]: `structured field`, `exact taxonomy term`, `reviewed alias`, or `ambiguous/hidden`.

If recall remains poor after alias fixes, a later NLP phase can use SkillSpan-style span extraction. Its authors found domain-adapted models significantly outperformed non-adapted models and single-task training outperformed their multi-task setup ([SkillSpan results](https://aclanthology.org/2022.naacl-main.366/)). That is evidence for a job-domain model, not for adding a general-purpose LLM API to this local TUI.

## Other facts to extract

Use a **structured-first, text-second** rule for every field. Schema.org defines `experienceRequirements`, including a structured `monthsOfExperience` form; `educationRequirements`; `employmentType`; `jobLocationType`; and `applicantLocationRequirements` for geographic limits on remote applicants ([Schema.org `JobPosting`](https://schema.org/JobPosting), [`experienceRequirements`](https://schema.org/experienceRequirements), [`applicantLocationRequirements`](https://schema.org/applicantLocationRequirements)). `jobLocationType` uses text such as `TELECOMMUTE` for remote jobs ([Schema.org `jobLocationType`](https://schema.org/jobLocationType)).

| Fact | Stored result | Honest rule |
|---|---|---|
| Remote mode | `remote`, `hybrid`, `on-site`, `unknown` | Trust an explicit structured field first. Use exact phrases as fallback. Do not turn “flexible work” into `hybrid`. Keep applicant-country restrictions separately. |
| Experience | minimum and maximum months, `required`/`preferred`, evidence | Parse explicit forms such as “3+ years” or “3–5 years”. Keep multiple requirements separately instead of selecting the largest number. Unknown stays unknown. |
| Seniority | `intern`, `entry/junior`, `mid`, `senior`, `lead/staff/principal`, `manager`, `unknown` | Prefer explicit title/description wording. Do not derive seniority only from experience years; show both dimensions. |
| Education | normalized credential level, subject, `required`/`preferred`, evidence | Prefer structured data. Preserve “or equivalent experience” because it changes the meaning of a degree requirement. |
| Employment type | normalized existing field plus `unknown` | Normalize source spellings but keep the raw value for inspection. |

Unknown is a result, not an error. Every chart must show its extraction coverage, for example: `Remote mode found in 812 / 1,200 postings (67.7%)`; percentages by remote mode should use **812**, not 1,200, while the unknown count remains visible.

## Deduplication

Repeated scans are already deduplicated by `(company_id, source_id)`, and snapshots are added only when `content_hash` changes ([schema](src/storage/schema.rs), [storage flow](src/storage/store.rs)). That is sufficient while each company has one official source.

Cross-source deduplication becomes necessary only if aggregators or duplicate company sources are added. Eurostat calls deduplication a basic condition for high-quality online-job-ad statistics because the same job is often published on several portals; its challenge distinguishes full, semantic, temporal, partial, and cross-language duplicates ([Eurostat deduplication challenge](https://cros.ec.europa.eu/book-page/results-web-intelligence-online-job-advertisement-oja-deduplication-challenge)).

The smallest safe future design is two stages:

1. Exact fingerprint of normalized company, title, location, and description, excluding URLs, source IDs, and retrieval dates.
2. Only for remaining candidates within the same company and a close publication window, a reviewed fuzzy comparison of title plus description.

Store a duplicate group and keep all source records; choose one representative only for counting. Do not delete evidence. Do not add fuzzy matching now: the current audit found no duplicate groups under the existing hash, and all companies currently use one official source.

## Counts, percentages, sample size, and coverage

For a selected scope and time window:

```text
skill share = unique postings mentioning the skill
              ------------------------------------
              unique postings with usable descriptions
```

Use **posting share**, not raw mention count: repeated marketing text or many mentions inside one description must not increase demand. Always show numerator, denominator, date window, company filter, job-state filter, taxonomy version, and extraction coverage.

These values exactly describe the tracked cache, but the cache is not a random sample of the Dutch labour market. Cedefop found coverage bias by sector, occupation, and geography, and the European Commission’s JRC reports that white-collar professional occupations are better represented while some basic digital skills are under-mentioned because employers may treat them as obvious ([Cedefop](https://www.cedefop.europa.eu/en/publications/6217), [JRC skills-intelligence findings](https://joint-research-centre.ec.europa.eu/projects-and-activities/employment/skills-intelligence-online-job-advertisements_en)).

Therefore:

- Show raw counts at every sample size; small counts are still true for this cache.
- Do not label percentage changes as significant or infer shortages.
- Do not add a conventional confidence interval: it would describe random-sampling error, but not the larger source-selection and missing-mention biases here. NIST explains confidence intervals as repeated-random-sample coverage and warns that small-sample proportion methods need special handling ([NIST confidence intervals](https://www.itl.nist.gov/div898/handbook/prc/section1/prc14.htm), [NIST proportion intervals](https://itl.nist.gov/div898/handbook/prc/section2/prc241.htm)).
- For co-occurrence and comparisons, display `low volume` when fewer than 10 postings support a result. **Ten is a conservative UI rule, not a statistical theorem**; it prevents a pair seen once from looking important.
- Show source health and scan completeness beside analytics. Cedefop notes that blocked or inaccessible scrapers create gaps and that text classification can vary across language and time ([Cedefop OJA limitations](https://www.cedefop.europa.eu/en/data-insights/utilising-online-job-advertisements-identify-labour-market-imbalances)).

## Time trends

Use **new-posting cohorts**, not the set of jobs active today:

- Primary time: `published_at` when present.
- Fallback time: `first_seen_at`, labelled “first seen”.
- Never silently combine the two timestamps in one series.
- Weekly buckets for fewer than 26 weeks of history; monthly buckets after that.
- Plot unique new postings, unique closed postings, and skill share among new postings.
- Extract facts from the snapshot associated with that period so later description edits do not rewrite history.

The current cache cannot yet show a genuine observed trend because all 1,200 jobs were first seen in one scan window. A chart grouped by their old `published_at` values would show only jobs that survived open until the first scan and would omit jobs that opened and closed earlier. Start the trend clock now and render “Collecting history — 1 weekly bucket” until at least four complete weekly buckets exist. **Four weeks is a product-readability rule, not proof of a stable trend.**

Cedefop uses quarterly refreshes and rolling four-quarter reporting for broad European OJA intelligence, but this local app can update weekly because it tracks a much smaller source set; neither cadence removes representativeness limits ([Cedefop Skills OVATE methodology](https://www.cedefop.europa.eu/de/projects/skills-online-job-advertisements)).

## Skill co-occurrence

For each unique posting, build the set of canonical skills and count every unordered pair once. Co-occurrence networks are a valid way to examine skills that employers request together; a recent peer-reviewed study built such a network from a curated, deduplicated set of 65 million UK adverts ([PLOS Complex Systems paper](https://journals.plos.org/complexsystems/article?id=10.1371/journal.pcsy.0000028)).

The MVP should not draw a node-link network. For the selected skill, rank related skills by:

```text
Jaccard(A, B) = postings containing both A and B
                ---------------------------------
                postings containing A or B
```

Show Jaccard similarity, pair count, and the selected-skill denominator. This corrects some popularity bias: a very common skill does not rank highly only because it appears often. Hide pairs below the 10-posting UI rule by default, with a control to reveal them.

Network centrality, clustering, and a custom graph can wait until the cache is much larger. The PLOS study used 65 million deduplicated adverts, 3,906 skills, dimensionality reduction, graph construction, and multiscale community detection; copying that visual language onto 1,200 postings would imply more structure than the data supports ([methods and scale](https://journals.plos.org/complexsystems/article?id=10.1371/journal.pcsy.0000028)).

## Ratatui chart choices

The installed dependency is Ratatui **0.30.2** ([Cargo manifest](Cargo.toml)). Native widgets cover the useful charts without a new dependency.

| Widget | Use here | Capabilities and constraints |
|---|---|---|
| [`BarChart`](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.BarChart.html) | Top skills, co-skills, remote mode, seniority, experience bands, companies | Horizontal/vertical and grouped bars; labels, displayed values, styles, widths, gaps, symbols, and maximum value. Data values are `u64`. Horizontal bars keep category labels readable. |
| [`Chart`](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Chart.html) + [`Dataset`](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Dataset.html) | Weekly/monthly posting and skill-share trends | Cartesian axes, legends, and `(f64, f64)` scatter, line, bar, and area datasets. Input must be sorted for line plots. Bounds and labels are manual. Ratatui documents that fewer than two axis labels are not rendered and more than three are positioned incorrectly, so use two or three. |
| [`Sparkline`](https://docs.rs/ratatui/0.30.2/ratatui/widgets/struct.Sparkline.html) | Tiny recent-trend summary beside a metric | Uses `u64` values, supports missing values, styles, direction, and custom symbols. It has no documented axes, labels, or legend, so it is a summary, not the main chart. |
| [`Canvas`](https://docs.rs/ratatui/0.30.2/ratatui/widgets/canvas/struct.Canvas.html) | Later custom heatmap or network only | Supports manual shapes, labels, layers, colours, and coordinate bounds. Braille gives 2×4 detail per terminal cell but depends on font support ([Ratatui markers](https://docs.rs/ratatui/0.30.2/ratatui/symbols/enum.Marker.html)). Geometry and mouse hit-testing remain application work. |

Avoid pie and donut charts in the MVP: Ratatui documents no native widget for them, so they would require manual `Canvas` work. Hover, tooltips, and click-to-filter also need the app’s own coordinate mapping because these widget APIs expose rendering and styling, not hit-testing.

### Recommended responsive layout

**Wide terminal**

```text
┌ Quality: 1,200 jobs · 100% descriptions · 9/9 healthy sources ┐
├ Top skills ──────────────────┬ Weekly observed postings ───────┤
│ horizontal bars              │ line/area chart                 │
├ Related to: selected skill ──┼ Work facts ─────────────────────┤
│ horizontal Jaccard bars      │ remote / seniority / experience │
└ Evidence and matching jobs for the selected result ────────────┘
```

**Medium terminal:** one chart plus the evidence pane, switched with tabs.  
**Narrow terminal:** ranked list, exact values, and a sparkline; open details as the existing single-pane UI already does.

Use the existing theme, one accent colour per series, a stronger selected state, muted axes, and the current Unicode/ASCII fallback. Ratatui supports ANSI, indexed, and RGB colours, but RGB appearance depends on terminal support, so the existing theme palette is safer for a released binary ([Ratatui `Color`](https://docs.rs/ratatui/0.30.2/ratatui/style/enum.Color.html)).

## Concrete delivery phases

### Phase 1 — realistic local MVP

- Add extraction coverage and source-health context to Analytics.
- Store versioned extracted facts per content hash so unchanged descriptions are not reprocessed.
- Normalize the existing skill aliases to ESCO URIs; add a reviewed English/Dutch ESCO term subset as a release-time asset, not a runtime API dependency.
- Add structured-first remote, experience, seniority, education, and employment facts with `Unknown` buckets and evidence.
- Add horizontal `BarChart` panels for top skills, selected-skill co-occurrence, and work facts.
- Add a 100-description gold set and extraction precision/recall checks.
- Keep every current filter and matching-job drill-down.

### Phase 2 — trends after data exists

- Extract facts for all stored snapshots, including closed jobs.
- Add weekly new/closed posting counts and skill shares with `Chart`.
- Show sparklines only after at least four complete weekly buckets.
- Add company and job-family comparison while keeping each denominator visible.

### Phase 3 — only after measured need

- Expand from the reviewed ESCO subset to broader ESCO coverage.
- Add a domain-adapted span model only if the gold set shows unacceptable recall after rule and alias fixes.
- Add fuzzy cross-source deduplication only when aggregators or overlapping sources create measured duplicates.
- Add a `Canvas` heatmap or network only if ranked co-skill bars fail to answer a real user question.

## What not to claim

- “Most demanded skill in the Netherlands” — the cache covers selected company sources, not a representative labour-market sample ([Cedefop coverage study](https://www.cedefop.europa.eu/en/publications/6217)).
- “Jobs requiring this skill grew 40%” — until comparable historical cohorts and complete scans exist.
- “Remote jobs are 30% of the market” — report “30% of classified tracked postings”; always show unknown coverage.
- “Skill shortage” — online ads observe employer demand, not worker supply, and cannot establish a shortage alone ([Cedefop limitations](https://www.cedefop.europa.eu/en/data-insights/utilising-online-job-advertisements-identify-labour-market-imbalances)).
- “No mention means not required” — JRC found that basic digital skills can be under-mentioned because employers may assume them ([JRC findings](https://joint-research-centre.ec.europa.eu/projects-and-activities/employment/skills-intelligence-online-job-advertisements_en)).

The result is useful and honest: **what the tracked employers say, how often they say it, what appears together, how the observed postings change, and exactly how much of the cache supports each chart.**
