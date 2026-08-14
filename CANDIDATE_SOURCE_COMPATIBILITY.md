# Candidate source compatibility

Research date: **2026-08-14**. Scope: the **79 candidate companies** in the approved backlog, excluding the 20 sources already enabled.

This is the **historical research snapshot used to plan implementation**. Its “Current adapter?” cells describe the repository at research time and are no longer the current support record. Use [SUPPORTED_COMPANIES.md](SUPPORTED_COMPANIES.md) for shipped support and [COMPANY_SOURCE_QUEUE.md](COMPANY_SOURCE_QUEUE.md) for delivery status.

## Decision rule

`Yes` means the company exposes a current first-party Ashby, Greenhouse, Jibe, or Recruitee board that fits a reusable adapter already in this repository. It still needs configuration, a fixture, a complete-scan test, and a live verification before it can be enabled.

`No` means the current official source uses another ATS, a custom site, LinkedIn only, or no verifiable public vacancy board. Company-specific adapters and the hard-coded Yuki Teamtailor feed are not treated as reusable. `Hold` means the source technology fits, but there is no safe current Netherlands location signal or the public board is not demonstrably the company's complete current source.

Netherlands vacancy observations are a snapshot and can change. `Yes` does not mean the vacancy matches the repository's title filters.

## Summary

- **Directly compatible:** Backbase, Da Vinci Derivatives, IMC Trading, Flow Traders, Stripe, bunq, Elastic, Miro, Checkout.com, Fourthline, MultiSafepay, Ockto, DRW, Jump Trading, Tower Research, WEBB Traders, ACT Commodities, STX Group, DPG Media.
- **Compatible but hold:** Bitvavo (all Ashby locations are only `Headquarters`), Jane Street and Wise (no current Netherlands vacancies; Wise's main careers site is Attrax rather than the small Greenhouse board).
- **New generic adapter required:** Lever for Finom; Personio for Silverflow and Ohpen; Workable for Keylane; Workday/SAP/other enterprise ATS sources listed below.
- **Custom or unclear source:** keep low priority until a complete, stable first-party feed is proved.

## Priority 1–2: global technology, fintech, and trading

| # | Company | Official current source | ATS/feed | NL vacancies now | Current adapter? | Confidence / caveat |
|---:|---|---|---|---|---|---|
| 1 | Backbase | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/workatbackbase/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active complete board. |
| 2 | Bitvavo | [Ashby API](https://api.ashbyhq.com/posting-api/job-board/bitvavo) | Ashby | **Unresolved** | **Hold** | High ATS confidence; all six postings currently say only `Headquarters`, so the adapter cannot safely prove Netherlands. |
| 3 | Da Vinci Derivatives | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/davinciderivatives/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active board with Amsterdam roles. |
| 4 | Uber | [Amsterdam careers](https://jobs.uber.com/en/locations/amsterdam/) | Uber custom careers | **Yes** | **No** | High; official Amsterdam page has open-role search, but no existing adapter matches it. |
| 5 | Optiver | [Amsterdam technology jobs](https://www.optiver.com/join-us/jobs/technology/amsterdam/) | Optiver custom careers | **Yes** | **No** | High; many Amsterdam roles, but the official site is not one of the reusable ATS adapters. |
| 6 | IMC Trading | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/imc/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; [official search](https://www.imc.com/eu/search-careers) shows Amsterdam roles and the active board is complete. |
| 7 | Flow Traders | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/flowtraders/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; official careers pages link to active Amsterdam vacancies. |
| 8 | Maven Securities | [Experienced hires](https://www.mavensecurities.com/experienced-hires/) | Custom careers | **Not confirmed** | **No** | Medium; official careers content exists, but no stable complete supported feed was verified. |
| 9 | Stripe | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/stripe/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active complete board has a Netherlands posting in this snapshot. |
| 10 | bunq | [Recruitee API](https://bunq.recruitee.com/api/offers/) | Recruitee | **Yes — 12/12** | **Yes** | High; [official Netherlands careers page](https://careers.bunq.com/positions/offices/netherlands) points to the bunq board. |
| 11 | Finom | [official careers](https://careers.finom.co/) / [Lever board](https://jobs.eu.lever.co/pnlfin) | Lever | **No in current 2-role feed** | **No** | High; official page links to Lever tenant `pnlfin`; repository has no Lever adapter. |
| 12 | Silverflow | [Personio board](https://silverflow.jobs.personio.com/) | Personio | **Yes** | **No** | High; official careers page links to Personio; repository has no Personio adapter. |
| 13 | Elastic | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/elastic/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active complete board, though the current Netherlands roles may not match target titles. |
| 14 | Miro | [Ashby API](https://api.ashbyhq.com/posting-api/job-board/miro) | Ashby | **Yes** | **Yes** | High; active complete board with Amsterdam entries. |
| 15 | TomTom | [official Amsterdam search](https://www.tomtom.com/careers/joboverview/?location=Amsterdam%2C+The+Netherlands) | TomTom custom careers | **Yes** | **No** | High; official page currently lists Amsterdam engineering work, but its search is not a supported ATS feed. |
| 16 | Checkout.com | [Ashby API](https://api.ashbyhq.com/posting-api/job-board/checkout.com) | Ashby | **Yes** | **Yes** | High; active complete board with Amsterdam entries. |
| 17 | Wise | [official careers](https://wise.jobs/) / [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/wise/jobs?content=true) | Attrax; small Greenhouse board also active | **No** | **Hold** | Medium; the Greenhouse endpoint works but has only two non-NL roles, while the main official site uses Attrax. Completeness/ownership must be proved before using Greenhouse. |
| 18 | Revolut | [Netherlands careers](https://www.revolut.com/en-NL/careers/) | Revolut custom careers | **No Netherlands location seen** | **No** | High source confidence; official site currently exposes hundreds of global roles but no reusable ATS feed. |
| 19 | Klarna | [branded job board](https://jobs.deel.com/klarna) | Deel jobs | **Yes** | **No** | High; current board has Amsterdam commercial/implementation roles, but Deel is unsupported. |
| 20 | Fourthline | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/fourthline/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active complete board has Netherlands entries. |
| 21 | flatexDEGIRO | [official careers](https://www.degiro.com/careers) | Custom corporate careers | **Yes** | **No** | High; Amsterdam roles are visible, but no supported complete ATS endpoint was verified. |
| 22 | Ohpen | [Personio board](https://ohpen.jobs.personio.com/) | Personio | **Yes** | **No** | High; active board has Amsterdam roles; repository has no Personio adapter. |
| 23 | Worldline | [official jobs](https://jobs.worldline.com/?lang=en) | SAP SuccessFactors careers | **Yes** | **No** | High ATS/source confidence; enterprise careers platform is unsupported. |
| 24 | Plaid | [official careers](https://plaid.com/careers/) | Custom/Contentful careers | **No NL role observed** | **No** | Medium; Plaid has a Dutch entity, but its current official job list did not expose a supported ATS or visible Netherlands role. |
| 25 | Buckaroo | [official vacancies](https://www.buckaroo.eu/about/vacancies) | Buckaroo website | **Yes** | **No** | High; current NL vacancy page is first-party but not a supported complete feed. |
| 26 | PAY. | [official careers](https://www.pay.nl/en/careers) | HubSpot-hosted company page | **Yes** | **No** | High; NL roles are rendered on one first-party page, but it is not a supported ATS and completeness semantics need work. |
| 27 | MultiSafepay | [Recruitee API](https://careers.multisafepay.com/api/offers/) | Recruitee | **Yes — 2/13** | **Yes** | High; active complete Recruitee board. |
| 28 | Knab | [official vacancies](https://www.werkenbijknab.nl/en/vacancies) | Custom careers; former Greenhouse board inactive | **Yes** | **No** | High for source; the previously indexed Greenhouse board now returns 404, so it must not be configured. |
| 29 | Brand New Day | [official careers](https://werkenbij.brandnewday.nl/) | Custom careers | **Yes** | **No** | High; current Amsterdam roles are visible, but no supported stable feed was verified. |
| 30 | FRISS | [official vacancies](https://www.friss.com/vacancies) | LinkedIn only | **Not verifiable from first-party feed** | **No** | High; FRISS explicitly says it posts all openings on LinkedIn. LinkedIn is not a complete first-party adapter source. |
| 31 | Keylane | [official jobs](https://careers.keylane.com/jobs/) / [application](https://apply.workable.com/keylane/) | Workable | **Yes** | **No** | High; current Utrecht/Rotterdam roles exist, but Workable is unsupported. |
| 32 | Currence / iDEAL | No current first-party vacancy board verified | Unknown | **Unknown** | **No** | Low; do not integrate from third-party listings. Recheck the current organisation/brand ownership before more work. |
| 33 | Payaut | No current first-party vacancy board verified | Unknown | **Unknown** | **No** | Low; official company presence was found, but not a stable public jobs source. |
| 34 | Ockto | [Recruitee API](https://ockto.recruitee.com/api/offers/) | Recruitee | **Yes — 5/5** | **Yes** | High; active complete board with Naarden roles. |
| 35 | DRW | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/drweng/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; [official jobs page](https://www.drw.com/work-at-drw/listings) shows Netherlands roles. |
| 36 | Jane Street | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/janestreet/jobs?content=true) | Greenhouse | **No** | **Hold** | High; active complete board, but no Netherlands role in the snapshot. |
| 37 | Jump Trading | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/jumptrading/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active complete board has Netherlands entries. |
| 38 | Tower Research | [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/towerresearchcapital/jobs?content=true) | Greenhouse | **Yes** | **Yes** | High; active complete board has Netherlands entries. |
| 39 | WEBB Traders | [official careers](https://www.webbtraders.com/careers) / [Recruitee API](https://webbtraders.recruitee.com/api/offers/) | Recruitee | **Yes** | **Yes** | High; active official Recruitee board with Amsterdam roles. |

## Priority 3–4: remaining companies

The second half of the backlog is recorded below after the same first-party-source check.

| # | Company | Official current source | ATS/feed | NL vacancies now | Current adapter? | Confidence / caveat |
|---:|---|---|---|---|---|---|
| 40 | ACT Commodities | [official vacancies](https://careers.actgroup.com/vacancies) / [Greenhouse board](https://job-boards.greenhouse.io/testendouble) | Greenhouse (`testendouble`) | **Yes — 4 observed** | **Yes** | High; the unusual board token was followed from the official careers flow. |
| 41 | STX Group | [official careers](https://stxgroup.com/careers/) / [Greenhouse API](https://boards-api.greenhouse.io/v1/boards/stxgroup/jobs?content=true) | Greenhouse | **Yes — 4 observed** | **Yes** | High; active complete board. |
| 42 | OTC Flow | [official careers](https://www.otcflow.com/careers) / [BambooHR board](https://otcflow.bamboohr.com/careers) | BambooHR | **Yes** | **No** | Medium; first-party careers links to BambooHR, which is unsupported. |
| 43 | Vitol | [official careers](https://www.vitol.com/careers/) / [SmartRecruiters board](https://jobs.smartrecruiters.com/Vitol) | SmartRecruiters | **No** | **No** | High; live complete feed had no Netherlands role in this snapshot. |
| 44 | Amazon / AWS | [Netherlands jobs](https://www.amazon.jobs/en/location/netherlands) | Amazon custom careers | **Yes** | **No** | High; proprietary platform is unsupported. |
| 45 | Google | [official job search](https://www.google.com/about/careers/applications/jobs/results/) | Google custom careers | **Yes** | **No** | High; Netherlands results include Eemshaven, but the proprietary platform is unsupported. |
| 46 | Microsoft | [Amsterdam jobs](https://careers.microsoft.com/v2/global/en/locations/amsterdam.html) | Microsoft custom careers | **Yes** | **No** | High; proprietary platform is unsupported. |
| 47 | Meta | [official job search](https://www.metacareers.com/jobs/) | Meta custom/login-gated careers | **Not verifiable** | **No** | Low; no current Netherlands vacancy could be safely observed from the public interface. |
| 48 | PostNL | [official careers](https://www.postnl.nl/werkenbij/) | PostNL custom careers | **Yes** | **No** | High; proprietary source is unsupported. |
| 49 | PGGM | [official vacancies](https://www.pggm.nl/werken-bij/vacatures) | Custom careers | **Yes** | **No** | High; current .NET vacancies were observed, but no reusable ATS feed was verified. |
| 50 | NS | [official vacancies](https://www.werkenbijns.nl/vacatures) | Hamilton CMS/custom feed | **Yes** | **No** | High; unsupported custom source. |
| 51 | Achmea | [official vacancies](https://www.werkenbijachmea.nl/vacatures) | Hamilton CMS/custom feed | **Yes** | **No** | High; unsupported custom source. |
| 52 | a.s.r. | [official vacancies](https://www.werkenbijasr.nl/vacatures) | Custom Vue/API | **Yes** | **No** | High; unsupported custom source. |
| 53 | Nationale-Nederlanden | [official careers](https://www.nn-careers.com/en/) | Custom careers | **Yes** | **No** | High; unsupported custom source. |
| 54 | Alliander | [official vacancies](https://werkenbij.alliander.com/vacatures) | Custom Next.js/API | **Yes** | **No** | High; unsupported custom source. |
| 55 | Exact | [official vacancies](https://www.exact.com/careers/vacancies) | Exact custom careers module | **Yes** | **No** | High; current .NET roles were observed, but the custom source is unsupported. |
| 56 | AFAS Software | [official vacancies](https://www.werkenbijafas.nl/alle-vacatures) | AFAS custom careers | **Yes** | **No** | High; unsupported custom source. |
| 57 | Wolters Kluwer | [official careers](https://careers.wolterskluwer.com/nl-nl) / [Workday board](https://wk.wd3.myworkdayjobs.com/External) | Workday | **Yes** | **No** | High; repository has no generic Workday adapter. |
| 58 | ChipSoft | [official vacancies](https://www.chipsoft.com/nl-NL/werken-bij/vacatures) | ChipSoft custom careers | **Yes** | **No** | High; current .NET roles were observed, but the source is unsupported. |
| 59 | BNG Bank | [official vacancies](https://www.bngbank.nl/werken-bij/vacatures) / [application portal](https://solliciterenbij.bngbank.nl/vacatures) | Custom recruitment API | **Yes** | **No** | High; unsupported custom source. |
| 60 | Schiphol Group | [official jobs](https://www.schipholcareers.nl/en/jobs/all-jobs) | SAP SuccessFactors/custom front | **Yes** | **No** | High; unsupported enterprise ATS. |
| 61 | KLM | [official jobs](https://careers.klm.com/en/jobs/) | KLM custom careers; ATS not exposed | **Yes** | **No** | Medium; Tech & Data vacancies are visible, but a stable complete feed was not verified. |
| 62 | ANWB | [official vacancies](https://www.werkenbijanwb.nl/vacatures) | Custom JSON (`/fuse/vacancies.json`) | **Yes — 166 observed** | **No** | High; usable custom data exists, but no current adapter matches its schema. |
| 63 | APG | [official vacancies](https://werkenbij.apg.nl/vacatures/) / [IT careers](https://werkenbij.apg.nl/en/it/) | WillHire | **Yes** | **No** | High; WillHire is unsupported. |
| 64 | UWV | [official vacancies](https://www.uwv.nl/nl/werken-bij/vacatures) | UWV custom careers | **Yes** | **No** | Medium; current NL-only jobs page exists, but no reusable complete feed was identified. |
| 65 | RDW | [official ICT jobs](https://www.werkenbijderdw.nl/vakgebieden/ict/) | RDW custom faceted-search API | **Yes** | **No** | High; unsupported schema. |
| 66 | Kadaster | [official careers](https://werkenbijhetkadaster.nl/) | Custom; ATS not confidently identified | **Yes** | **No** | Medium; vacancies exist, but a stable complete feed needs more investigation. |
| 67 | ProRail | [official vacancies](https://www.werkenbijprorail.nl/vacatures) / [ICT careers](https://www.werkenbijprorail.nl/en/jouw-expertise/ict) | ProRail custom careers | **Yes — 17 observed** | **No** | High; unsupported custom source. |
| 68 | IVO Rechtspraak | [official IVO careers](https://werkenbijderechtspraak.nl/ivo-rechtspraak/) / [vacancies](https://werkenbijderechtspraak.nl/vacatures/) | WordPress/custom jobs integration | **Unclear** | **No** | Medium; Rechtspraak vacancies are visible, but the exact current IVO subset was not safely resolved. |
| 69 | Defensie / JIVC | [official COMMIT/JIVC vacancies](https://werkenbijdefensie.nl/vacatures/niet-militaire-vacatures/commit) | Defensie custom careers | **Yes — 75 COMMIT roles observed** | **No** | High; unsupported custom source. |
| 70 | Stedin | [official vacancy search](https://werkenbij.stedin.net/vacatures-zoeken) | Radancy/custom careers | **Yes — 108 observed** | **No** | Medium/high; IT/Data roles exist, but the platform is unsupported. |
| 71 | TenneT | [official careers](https://karriere.tennet.eu/) / [Avature search](https://careers.tennet.eu/en_US/careers/SearchJobs) | Avature | **Yes** | **No** | High; Avature is unsupported. |
| 72 | Gasunie | [official vacancies](https://www.werkenbijgasunie.nl/vacatures) | Custom API (`/api/vacancy/...`) | **Yes** | **No** | High; unsupported schema. |
| 73 | KPN | [official vacancies](https://jobs.kpn.com/vacatures/search/vacatures) | Custom API (`/api/vacancy/...`) | **Yes** | **No** | High; Tech & IT roles exist, but the custom schema is unsupported. |
| 74 | DPG Media | [official vacancies](https://vacatures.dpgmedia.nl/) / [Recruitee API](https://vacatures.dpgmedia.nl/api/offers/) | Recruitee | **Yes — 85 observed** | **Yes** | High; active complete board has Netherlands technology roles. |
| 75 | Vanderlande | [official careers](https://careers.vanderlande.com/) / [NL jobs](https://careers.vanderlande.com/nl/alle-vacatures/) | Workday behind custom WordPress front | **Yes** | **No** | High; Workday is unsupported. |
| 76 | Lely | [official vacancies](https://www.lely.com/nl/vacatures/) | Custom Optimizely/Episerver | **Yes — 104 observed** | **No** | High; unsupported custom source. |
| 77 | Planon | [official vacancies](https://planonsoftware.com/us/careers/vacancies/) / [Talentsoft board](https://planon-candidate.talent-soft.com/stelle/stellenliste.aspx?facet_Country=91&lcid=2057&showSearchUrl=1) | Talentsoft | **Yes — 19 observed** | **No** | High; Talentsoft is unsupported. |
| 78 | ilionx | [official vacancies](https://werkenbij.ilionx.com/vacatures) | Teamtailor behind custom domain | **Yes** | **No** | High; the existing Teamtailor/Yuki implementation is hard-coded to Yuki and cannot be configured for ilionx. |
| 79 | Info Support | [official careers](https://werkenbij.infosupport.com/en) / [JSON feed](https://werkenbij.infosupport.com/en/jobs.json) | Teamtailor JSON feed | **Yes** | **No** | High; schema is close, but the existing Yuki source hard-codes Yuki identity, host, and feed assumptions. |

## Pragmatic integration order

1. **Existing adapters, live NL roles:** Backbase, Da Vinci Derivatives, IMC Trading, Flow Traders, Stripe, bunq, Elastic, Miro, Checkout.com, Fourthline, MultiSafepay, Ockto, DRW, Jump Trading, Tower Research, WEBB Traders, ACT Commodities, STX Group, DPG Media.
2. **Existing adapters, blocked/low yield:** Bitvavo after official `Headquarters = Netherlands` proof; Jane Street and Wise only when a current Netherlands role and complete-board ownership are verified.
3. **One useful new generic adapter at a time:** Personio unlocks Silverflow and Ohpen; Lever unlocks Finom; then consider Workable, Workday, and Teamtailor only if their live NL role yield justifies the work.
4. **Custom sites last:** keep them in the backlog; build a company-specific source only when a live target vacancy exists and the endpoint proves complete pagination.
