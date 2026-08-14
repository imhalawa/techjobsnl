# Company expansion research

Research date: **2026-08-14**. Scope: Netherlands roles in software, platform/SRE, data/ML, and security engineering. Only first-party careers/ATS sources, company legal pages, and the official IND register were used.

## Recommendation

Start with **DataSnipper, Databricks, and Reddit**. They combine relevant live vacancies, an IND-listed Dutch sponsor candidate, and a complete official board supported by an existing adapter. Add **WeTravel** and **Axual** next. Treat **Bitvavo** as promising but blocked until its `Headquarters` ATS location can be proved to mean the Netherlands without a manual assumption.

Do not start with a new scraper. Every shortlisted company below has an Ashby, Greenhouse, or Recruitee board already handled by this repository. This is less work and gives stronger completeness checks than adding a custom HTML source.

## How the ranking works

- **Fit**: similarity to the current mix of fintech, marketplace, SaaS/platform, and large-scale data companies, plus roles matching the target families.
- **Current evidence**: live Netherlands-aligned vacancies observed in the complete official board on 2026-08-14. The main count uses the repository's current title filters; useful adjacent roles are stated separately. Counts are a snapshot and can change at any time.
- **Sponsor candidate**: an exact organisation name and KvK number in the [official IND public register for work](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Legal caveat**: an IND listing proves that the listed entity is a recognised sponsor. It does **not** prove that this entity will employ the applicant. Unless a vacancy names the legal employer and matching KvK number, the employment entity must be confirmed before relying on sponsorship.
- **Integration effort**: relative implementation effort using the adapters already in this repository. `Low` means the source type exists; it still needs configuration, fixtures, and one complete-scan test before enabling.

## Ranked shortlist

| Rank | Company | Why it is close | Current Netherlands evidence | Complete official source | Sponsor candidate | Integration |
|---:|---|---|---|---|---|---|
| 1 | **DataSnipper** | Amsterdam B2B SaaS, AI/document processing, and platform reliability; close to Funda/Mollie-style product engineering | **5 filter-matching roles** in Amsterdam: SRE and four software/AI roles; two aligned manager/director roles are excluded by current filters | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/datasnipper) | `DataSnipper B.V.`, KvK `69343861` | **Low** — existing Ashby adapter |
| 2 | **Databricks** | Large data/AI platform with backend, full-stack, security/cloud, and distributed-data work | **7 filter-matching technical roles** in Amsterdam; additional manager, privacy, forward-deployed, and security/cloud roles are adjacent | [Greenhouse board API](https://boards-api.greenhouse.io/v1/boards/databricks/jobs?content=true) | `Databricks`, KvK `51208121` | **Low** — existing Greenhouse adapter |
| 3 | **Reddit** | Large consumer platform with ML, SRE, ads, and security work; close to eBay, Booking.com, and bol.com scale | **4 filter-matching roles** in the Netherlands: one SRE, two ML, and one security role | [Greenhouse board API](https://boards-api.greenhouse.io/v1/boards/reddit/jobs?content=true) | `Reddit Netherlands B.V.`, KvK `83433880` | **Low** — existing Greenhouse adapter |
| 4 | **WeTravel** | Amsterdam travel-payments platform; directly overlaps Booking.com, Mollie, Airwallex, and Adyen | **1 filter-matching platform role**; one additional payments product-engineer role is aligned but does not match the current regex | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/wetravel) | `WETRAVEL B.V.`, KvK `70669783` | **Low** — existing Ashby adapter |
| 5 | **Axual** | Utrecht event-streaming product built around Kafka and Kubernetes; strong backend/platform match | **1 filter-matching Utrecht role**: Backend (Full-stack) Software Engineer | [Recruitee board API](https://axual.recruitee.com/api/offers/) | `Axual B.V.`, KvK `64350622` | **Low** — existing Recruitee adapter |
| 6 | **Bitvavo** | Amsterdam crypto trading/payments platform; close to Adyen, Mollie, and Airwallex | **2 title matches**: Senior Data Platform Engineer and Senior Platform Engineer. The ATS says only `Headquarters`, so Netherlands eligibility is not safely resolved yet | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/bitvavo) | `Bitvavo B.V.`, KvK `68743424` | **Medium** — adapter exists, but location proof/mapping is needed |
| 7 | **Miro** | Amsterdam SaaS collaboration platform at international scale; similar product/platform environment to Booking.com and bol.com | **0 current-filter matches**; one aligned Amsterdam application-security manager is excluded by the manager rule | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/miro) | `RealtimeBoard`, KvK `71057153` | **Low** — existing Ashby adapter |
| 8 | **Elastic** | Search, observability, and security platform; strong SRE/data/security adjacency | **0 filter-matching roles** in the Netherlands snapshot; **2 solution-architect roles** were aligned but outside the core target | [Greenhouse board API](https://boards-api.greenhouse.io/v1/boards/elastic/jobs?content=true) | `elasticsearch B.V.`, KvK `54656230` | **Low** — existing Greenhouse adapter |
| 9 | **Checkout.com** | Global payments platform and the closest direct peer of Adyen, Mollie, and Airwallex | **0 filter-matching roles** in Amsterdam; the current board had only **2 Amsterdam commercial roles** | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/checkout.com) | `Checkout Group B.V.`, KvK `74268341` | **Low** — existing Ashby adapter |
| 10 | **Tebi** | Amsterdam restaurant operating and payments platform founded by an Adyen co-founder; relevant payments/backend domain | **0 Netherlands vacancies** in the current 8-job board; all live roles were outside the Netherlands | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/tebi) | `Tebi B.V.`, KvK `82179204` | **Low** — existing Ashby adapter |
| 11 | **Hopper** | Travel-commerce and fintech platform; close to Booking.com and payments companies | **0 filter-matching Netherlands roles**; the board had **1 Netherlands-remote payroll role** | [Ashby board API](https://api.ashbyhq.com/posting-api/job-board/hopper) | `Hopper Netherlands B.V.`, KvK `84054778` | **Low** — existing Ashby adapter |

## Candidate evidence and caveats

### 1. DataSnipper

- The official Ashby endpoint returned **32 total vacancies**, of which **24 were in Amsterdam**. Five passed the current title filters: `Site Reliability Engineer`, `Senior Software Engineer, AI Agents`, `Senior Full-Stack Engineer, Agent Experiences`, `Senior Software Engineer (Financial Statement Suite)`, and `Software Engineer - Document Intelligence`. `Engineering Manager, Excel Agents` and `Director of Information Security` are aligned but deliberately excluded by the current manager/director filter: [official board](https://api.ashbyhq.com/posting-api/job-board/datasnipper).
- The IND register lists `DataSnipper B.V.` with KvK `69343861`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work). DataSnipper's official terms identify the Amsterdam entity with the same KvK number: [DataSnipper terms](https://www.datasnipper.com/pdf-proxy.pdf?url=https%3A%2F%2Feu-assets.contentstack.com%2Fv3%2Fassets%2Fbltc08aa646f32b9827%2Fblt2109238ccab969a1%2F696a12bc34206b2e8465af0d%2FDataSnipper_Terms_and_Conditions_-_Version_2025-07-01.pdf).
- **Caveat:** the board identifies the brand and Amsterdam location, but does not prove that every contract is with `DataSnipper B.V.`. Confirm the offer entity.

### 2. Databricks

- The Greenhouse endpoint returned **25 Netherlands-location vacancies**. Seven passed the current title filters: `Senior Software Engineer - Backend`, `Senior Software Engineer - Fullstack`, `Senior Staff Software Engineer - Delta`, `Senior Staff Software Engineer - Unity Catalog Runtime Enforcement`, `Software Engineer - Backend`, `Software Engineer - Fullstack`, and `Staff Software Engineer - Backend`. Manager, privacy, forward-deployed, and security/cloud titles are relevant adjacent roles but do not pass the current filters: [official board](https://boards-api.greenhouse.io/v1/boards/databricks/jobs?content=true).
- The IND register lists `Databricks` with KvK `51208121`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** the register uses `Databricks` rather than a legal-form suffix, and the vacancies name only the Databricks brand. Confirm the exact Dutch employment entity and KvK number before relying on sponsor status.

### 3. Reddit

- The Greenhouse endpoint returned **7 Netherlands vacancies**. Four passed the current title filters: `Lead Physical Security Engineer`, `Senior ML Engineer, Ads Foundational Representations`, `Staff Machine Learning Engineer, ML Efficiency`, and `Staff Site Reliability Engineer, Ads`: [official board](https://boards-api.greenhouse.io/v1/boards/reddit/jobs?content=true).
- The IND register lists `Reddit Netherlands B.V.` with KvK `83433880`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** `Lead Physical Security Engineer` passes the broad `security engineer` expression but may not be the application/product-security work intended by the user. The ATS names Reddit, not the Dutch legal employer; confirm the contract entity.

### 4. WeTravel

- The Ashby endpoint returned **24 total vacancies** and **6 Amsterdam-office vacancies**. `Senior Software Engineer, Platform` passes the current filters. `Senior Product Engineer - Payments Team` is strongly aligned but does not pass the current regex: [official board](https://api.ashbyhq.com/posting-api/job-board/wetravel).
- The IND register lists `WETRAVEL B.V.` with KvK `70669783`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** the postings name WeTravel, not the employer's registered legal name and KvK. Confirm that `WETRAVEL B.V.` issues the contract.

### 5. Axual

- The complete Recruitee endpoint returned **2 vacancies**. `Backend (Full-stack) Software Engineer` was a direct match in Utrecht: [official board](https://axual.recruitee.com/api/offers/).
- The IND register lists `Axual B.V.` with KvK `64350622`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** the careers board uses the Axual brand but does not establish the legal employer for the eventual contract. Confirm the employment entity.

### 6. Bitvavo

- The Ashby endpoint returned **6 vacancies**, including `Senior Data Platform Engineer` and `Senior Platform Engineer`. Both locations are only `Headquarters`, not a country or city: [official board](https://api.ashbyhq.com/posting-api/job-board/bitvavo).
- The IND register lists `Bitvavo B.V.` with KvK `68743424`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work). Bitvavo's official company-details page gives the same legal name and KvK and locates the business in Amsterdam: [Bitvavo company details](https://support.bitvavo.com/hc/en-us/articles/4405241847569-Company-details-Bitvavo).
- **Caveat:** the company address does not prove that Ashby's `Headquarters` means Amsterdam or that `Bitvavo B.V.` employs each role. Do not silently map it. Ask Bitvavo or find vacancy-level official evidence first.

### 7. Miro

- The Ashby endpoint returned **43 total vacancies** and **10 Amsterdam vacancies**. `Senior Manager – Application Security` is aligned but excluded by the current manager rule, leaving no filter match: [official board](https://api.ashbyhq.com/posting-api/job-board/miro).
- The IND register lists `RealtimeBoard` with KvK `71057153`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work). Miro's official subprocessor document names `RealtimeBoard B.V.` as its Netherlands affiliate: [Miro affiliates](https://miro.com/legal/documents/Miro-Current-Subprocessors-List.pdf).
- **Caveat:** the official sources connect Miro with a Dutch RealtimeBoard affiliate, but the vacancy does not state that this entity is the employer. The register's name also omits `B.V.`. Confirm the contract entity and matching KvK.

### 8. Elastic

- The Greenhouse endpoint returned **252 total vacancies**. Six were tagged Netherlands, but none passed the target filters. Two `Senior Solution(s) Architect` roles were technically adjacent; the other four entries were repeated `Business Strategy Developer` listings: [official board](https://boards-api.greenhouse.io/v1/boards/elastic/jobs?content=true).
- The IND register lists `elasticsearch B.V.` with KvK `54656230`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** Elastic vacancies do not establish that `elasticsearch B.V.` is the employer. This is a useful watch source, not a current high-yield source.

### 9. Checkout.com

- The Ashby endpoint returned **181 total vacancies**. Only two were in Amsterdam, and both were commercial rather than target engineering roles: [official board](https://api.ashbyhq.com/posting-api/job-board/checkout.com).
- The IND register lists `Checkout Group B.V.` with KvK `74268341`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** the global Checkout.com brand and Dutch sponsor candidate are not enough to establish the employer for a future Amsterdam engineering vacancy. Confirm it vacancy by vacancy.

### 10. Tebi

- The Ashby endpoint returned **8 vacancies**, all outside the Netherlands on the research date: [official board](https://api.ashbyhq.com/posting-api/job-board/tebi).
- The IND register lists `Tebi B.V.` with KvK `82179204`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** there is no current Netherlands vacancy to connect to this sponsor. Keep it as a cheap watch candidate, not an immediate source of applications.

### 11. Hopper

- The Ashby endpoint returned **37 total vacancies**. The only Netherlands-tagged role was `Senior Manager, Global Payroll Systems & Equity Operations`, which is outside the target: [official board](https://api.ashbyhq.com/posting-api/job-board/hopper).
- The IND register lists `Hopper Netherlands B.V.` with KvK `84054778`: [IND register](https://ind.nl/en/public-register-recognised-sponsors/public-register-work).
- **Caveat:** the current Netherlands-remote posting does not identify its legal employer. A future engineering posting would still need contract-entity confirmation.

## Proposed implementation order after approval

1. Add **DataSnipper**, **Databricks**, and **Reddit** using existing adapters.
2. Add **WeTravel** and **Axual** for travel-payments and data-platform coverage.
3. Hold **Bitvavo** until `Headquarters` has official Netherlands evidence or the adapter can resolve it from trustworthy vacancy data.
4. Add **Miro**, **Elastic**, **Checkout.com**, **Tebi**, and **Hopper** only as low-cost watch sources. They had no current-filter Netherlands vacancy in this snapshot.

No candidate should be presented as sponsorship-safe solely because the brand has a related IND-listed entity. The legal employer on the eventual offer is the controlling evidence.
