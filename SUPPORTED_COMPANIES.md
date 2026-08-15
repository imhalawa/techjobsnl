# Supported companies and roadmap

These companies supply the official vacancies used for direct job review and skill-based job discovery in TechJobsNL. Coverage is limited to enabled, verified sources.

Release snapshot: **2026-08-14**.

- **65 supported companies** are enabled and have a tested source adapter.
- **1 source is disabled.**
- **33 companies are on the roadmap.** They are not configured until a complete first-party source can be verified.

## Supported now

| Company | Adapter |
|---|---|
| ABN AMRO | Getnoticed |
| Achmea | Hamilton HTML / JSON-LD |
| ACT Commodities | Greenhouse |
| Adyen | Greenhouse |
| AFAS Software | AFAS HTML / JSON-LD |
| Airwallex | Ashby |
| Albert Heijn Tech | Albert Heijn API |
| Amazon / AWS | Amazon Jobs API |
| ANWB | ANWB Fuse JSON |
| Backbase | Greenhouse |
| Bitvavo | Ashby |
| bol.com | bol.com API |
| Booking.com | Jibe |
| Brand New Day | Getnoticed |
| Buckaroo | HTML + sitemap |
| bunq | Recruitee |
| Centric | Recruitee |
| Checkout.com | Ashby |
| ChipSoft | ChipSoft HTML |
| CM.com | Recruitee |
| Da Vinci | Greenhouse |
| Databricks | Greenhouse |
| DataSnipper | Ashby |
| DPG Media | Recruitee |
| DRW | Greenhouse |
| eBay | eBay Jobs API |
| Elastic | Greenhouse |
| Eneco | Eneco HTML / JSON-LD |
| Exact | Exact HTML / JSON-LD |
| Finom | Lever |
| flatexDEGIRO | SAP SuccessFactors |
| Flow Traders | Greenhouse |
| Fourthline | Greenhouse |
| Funda | Recruitee |
| Google | Google Jobs SSR |
| IMC Trading | Greenhouse |
| Info Support | Teamtailor |
| ING | ING HTML / JSON-LD |
| Jump Trading | Greenhouse |
| Keylane | Workable |
| Klarna | Deel Jobs |
| Maven Securities | Greenhouse |
| Microsoft | Eightfold API |
| Miro | Ashby |
| Mollie | Ashby |
| MultiSafepay | Recruitee |
| NS | Hamilton HTML / JSON-LD |
| Ockto | Recruitee |
| Ohpen | Personio |
| PAY. | PAY. / Nmbrs HTML |
| PGGM | PGGM HTML |
| PostNL | PostNL API |
| Rabobank | Rabobank API |
| Reddit | Greenhouse |
| Silverflow | Personio |
| STX Group | Greenhouse |
| TomTom | Lever |
| Topicus | Getnoticed |
| Tower Research | Greenhouse |
| Uber | Oracle HCM |
| Vanderlande | Workday |
| WEBB Traders | Recruitee |
| Wolters Kluwer | Workday |
| Worldline | SAP SuccessFactors API |
| Yuki | Teamtailor |

## Disabled

- Coolblue

## Roadmap

Order follows the release priority. A company moves to supported only after implementation, offline tests, a complete live verification, and a green full check.

| Priority | Company | Current blocker |
|---:|---|---|
| 1 | Optiver | Its API declared 31 Amsterdam jobs, but one declared first-party detail returned HTTP 404. |
| 2 | Stripe | The compatible Greenhouse board currently has no Netherlands role. |
| 3 | Wise | No current NL role; Attrax completeness is not proved. |
| 4 | Revolut | Cloudflare returns HTTP 403 to the board and its first-party data endpoint. |
| 5 | Plaid | No current NL role or complete supported source was observed. |
| 6 | Knab | Its official board and API currently declare zero vacancies. |
| 7 | FRISS | The company points to LinkedIn; no complete first-party feed was found. |
| 8 | Currence / iDEAL | No current first-party vacancy board is verified; ownership also needs rechecking. |
| 9 | Payaut | No stable public first-party jobs source was found. |
| 10 | Jane Street | Its complete Greenhouse board currently has no Netherlands role. |
| 11 | OTC Flow | Its first-party careers page links to unsupported BambooHR. |
| 12 | Vitol | Its complete SmartRecruiters feed currently has no Netherlands role. |
| 13 | Meta | The public/login-gated interface does not safely expose a complete NL feed. |
| 14 | a.s.r. | Its live roles use an unsupported custom Vue/API source. |
| 15 | Nationale-Nederlanden | Its live roles use an unsupported custom careers source. |
| 16 | Alliander | Its live roles use an unsupported custom Next.js/API source. |
| 17 | BNG Bank | Its vacancies use an unsupported custom recruitment API. |
| 18 | Schiphol Group | Its roles use SAP SuccessFactors behind a custom front end. |
| 19 | KLM | Tech and Data roles are visible, but no stable complete feed is verified. |
| 20 | APG | Its live roles use unsupported WillHire. |
| 21 | UWV | Its NL careers page exists, but no complete reusable feed is identified. |
| 22 | RDW | Its ICT roles use an unsupported custom search API. |
| 23 | Kadaster | Its vacancies exist, but the ATS and complete feed are not confidently identified. |
| 24 | ProRail | Its roles use an unsupported custom careers source. |
| 25 | IVO Rechtspraak | The current IVO subset cannot be resolved safely. |
| 26 | Defensie / JIVC | COMMIT/JIVC roles use an unsupported custom careers source. |
| 27 | Stedin | Its IT/Data roles use unsupported Radancy/custom careers. |
| 28 | TenneT | Its live roles use unsupported Avature. |
| 29 | Gasunie | Its live roles use an unsupported custom vacancy API. |
| 30 | KPN | Its Tech and IT roles use an unsupported custom vacancy API. |
| 31 | Lely | Its live roles use unsupported Optimizely/Episerver. |
| 32 | Planon | Its live NL roles use unsupported Talentsoft. |
| 33 | ilionx | Its official site is live, but a complete Teamtailor JSON feed is not proved. |

Detailed first-party evidence and caveats are in [SOURCES.md](SOURCES.md). Delivery history is in [COMPANY_SOURCE_QUEUE.md](COMPANY_SOURCE_QUEUE.md).
