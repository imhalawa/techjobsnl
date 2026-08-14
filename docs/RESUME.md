# Resume entry

Yes, you can list TechJobsNL in your Projects section if you built or materially contributed to it and can explain the design choices. Keep the claims tied to the current repository; do not present changing vacancy counts or inferred visa-sponsor mappings as guarantees.

## Recommended entry

**TechJobsNL — Rust, Tokio, Ratatui, SQLite, Reqwest**
Local terminal application for tracking and analysing Netherlands technology vacancies from official company career sources.

- Built a Rust terminal workflow for scanning 60+ company profiles across 30+ official-source strategies, with search, country/title filters, applied tracking, vacancy history, and responsive keyboard/mouse navigation.
- Designed complete/incomplete/failed scan semantics so partial source responses cannot falsely close jobs; isolated company failures with bounded concurrency, retries, timeouts, and SQLite transactions.
- Implemented explainable local analytics for skills, technology stacks, roles, seniority, experience, work mode, and recommendations, backed by exact posting evidence and deterministic offline tests.

## Short version

**TechJobsNL — Rust, Ratatui, SQLite**

- Built a local terminal job tracker for 60+ company profiles with safe scan lifecycle handling, filters, applied/history tracking, and evidence-backed job-market analytics.

## Claims to avoid

- Do not call it a complete view of the Netherlands labour market; it covers configured sources and filters.
- Do not claim every company sponsors visas; a brand-to-legal-entity mapping can be uncertain and sponsor status changes.
- Do not quote the active-job count from a screenshot; it is volatile.
- Do not say AI powers the product by default; analytics is local, and Claude/Codex discovery is optional and review-gated.
- Do not say live tests always pass; they are ignored by default and external contracts can drift.

## Be ready to explain

- Why incomplete scans preserve jobs instead of applying partial results.
- How stable `(company_id, source_id)` identity supports closing and reopening vacancies.
- Why analytics stores exact evidence and versions cached extraction.
- How background tasks keep database reloads, scans, and analytics off the UI path.
- How official-source adapters prove pagination and identity before returning Complete.
- Why optional AI suggestions require strict validation and human approval.

Before sending the resume, recount the current company and strategy totals from `config.toml` if the catalog changes.
