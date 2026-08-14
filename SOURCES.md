# Source and sponsor evidence

## Brand New Day

- Official source: <https://werkenbij.brandnewday.nl/vacatures>. Its first-party Getnoticed API declares the exact total, current page, page size, and page count. The adapter verifies every page, unique numeric vacancy ID, matching canonical detail, `JobPosting` publication date, full description, explicit Netherlands country, hiring brand, and official application path before accepting a complete scan. The site's <https://werkenbij.brandnewday.nl/robots.txt> allows crawling.
- Live discovery: the API declared 11 jobs across two pages on 2026-08-14. All 11 were in Amsterdam and every detail matched. The count is volatile and is not hardcoded in the smoke test.
- Brand New Day describes itself as an online Dutch pension bank for saving and investing and reports almost 300 colleagues: <https://werkenbij.brandnewday.nl/>. The configured `200+` band is conservative because the source does not publish an exact headcount.
- The current board and most vacancy text are in Dutch. Language, work-authorisation, relocation, and visa sponsorship conditions vary by vacancy; confirm them before applying.
- The board identifies the Brand New Day brand, not the legal contract entity for each vacancy. Confirm that entity before relying on recognised-sponsor status.

## Worldline

- Official source: <https://jobs.worldline.com/search/?q=&locationsearch=Netherlands&locale=en_US>. Its first-party SAP SuccessFactors endpoint at <https://jobs.worldline.com/services/recruiting/v1/jobs> declares the exact Netherlands total and exposes fixed 25-row pagination. The adapter obtains the required CSRF token from the official search page, validates every unique ID and explicit Netherlands location, then matches each canonical detail URL, title, date, full description, and official application path before accepting a complete scan.
- Live discovery: the endpoint declared 6 Netherlands vacancies on 2026-08-14. All 6 details matched. The count is volatile and is not hardcoded in the smoke test.
- Worldline describes itself as a European payment-services leader focused on payments and reports 14,500+ professionals: <https://worldline.com/en/home/top-navigation/about-worldline/who-we-are>. The configured `2,000+` band follows the app's EU large-company classification.
- The current Netherlands feed is in English, but language, work-authorisation, relocation, and visa sponsorship conditions vary by vacancy. The board does not prove sponsorship; confirm it on the specific role.
- Worldline's office directory lists several Netherlands group entities: <https://worldline.com/en/home/main-navigation/git/office-locations>. The vacancy board identifies the Worldline brand, not the legal contract entity; confirm that entity before relying on recognised-sponsor status.

## Google

- Official source: <https://www.google.com/about/careers/applications/jobs/results/?company=Google&location=Netherlands&sort_by=date>. Google's first-party server-rendered search data declares the exact total and fixed 20-row pagination. The adapter verifies every page, unique numeric job IDs and application URLs, Google ownership, publication timestamps, descriptions, and explicit `NL` location codes before accepting a complete scan.
- Live discovery: the source declared 20 Google jobs matching Netherlands on 2026-08-14. All 20 had explicit Netherlands locations, unique IDs, official job and application URLs, descriptions, and valid timestamps. The count is volatile and is not hardcoded in the smoke test.
- Alphabet reported 183,323 employees at year-end 2024: <https://abc.xyz/assets/99/21/46cafdba41089a12a2d86ea47d44/goog026-annualreport2024-web.pdf>. The configured `2,000+` band follows the app's EU large-company classification.
- The configured industries cover Google Services and Google Cloud: software, cloud computing, artificial intelligence, digital advertising, and consumer technology. Alphabet describes these businesses and products at <https://abc.xyz/investor/faqs-and-general-information/default.aspx>.
- The careers source is in English, but language, relocation, work-authorisation, and visa sponsorship conditions vary by vacancy. Confirm them on each role.
- The board identifies the Google brand, not the legal contract entity for each vacancy. Confirm that entity before relying on recognised-sponsor status.

## Knab — deferred

- Official source: <https://www.werkenbijknab.nl/vacatures>. Its first-party vacancy-search API at <https://www.werkenbijknab.nl/api/1/vacancy-search> declared exactly 0 published vacancies on 2026-08-14, matching the server-rendered board. The site's `robots.txt` permits this path, so automated-access denial is not the blocker.
- The former `knab`, `knabnl`, and `werkenbijknab` Greenhouse board tokens all returned HTTP 404 on 2026-08-14. No BAWAG first-party careers replacement for Knab was found.
- Knab remains unconfigured until a vacancy is published. A real listing and detail are required to prove stable IDs, official URLs, Netherlands locations, descriptions, dates, and exact completeness before enabling an adapter.

## flatexDEGIRO

- Official source: <https://jobs.flatexdegiro.com/search/?q=&locationsearch=NL>. The SAP SuccessFactors board declares the exact Netherlands total and page size. The adapter follows every declared page, requires unique numeric IDs, fetches every detail, and validates the title, explicit `NL` address, publication date, full description, employer marker, job URL, and official application URL before accepting a complete scan.
- flatexDEGIRO reports more than 1,200 employees across Europe: <https://flatexdegiro.com/English/esg/social/default.aspx>. It describes itself as a pan-European online broker that operates proprietary technology and a regulated bank: <https://www.flatexdegiro.com/>. The configured `1,000+` band follows the app's EU large-company classification.
- The Netherlands board currently contains English-language roles and roles that explicitly require Dutch. Language, relocation, work-authorisation, and visa sponsorship conditions are role-specific; confirm them on each vacancy.
- The ATS metadata still names `flatexDEGIRO AG`, while current corporate pages and vacancy text name `flatexDEGIRO SE`. Confirm the contract entity before relying on recognised-sponsor status.

## Klarna

- Official source: <https://jobs.deel.com/klarna>. The server-rendered board publishes a complete ordered ItemList. The adapter validates every unique official URL, fetches every listed JobPosting detail, matches its Deel ATS ID and canonical URL, and keeps only explicit Amsterdam vacancies.
- Klarna's official press kit reports 3,400 employees: <https://www.klarna.com/international/press/>. The configured `2,000+` band follows the app's EU large-company classification.
- Klarna describes itself as a digital bank and flexible-payments provider: <https://investors.klarna.com/overview/>. The configured industries cover its payments, banking, shopping, and merchant-commerce products.
- The current Amsterdam vacancy is in English. The board does not prove visa sponsorship or the legal contract entity; confirm both before relying on recognised-sponsor status.

## Microsoft

- Official source: <https://apply.careers.microsoft.com/careers?location=Netherlands&hl=en>. Its first-party search API at <https://apply.careers.microsoft.com/api/pcsx/search?domain=microsoft.com&query=&location=Netherlands&start=0&hl=en> declares an exact total and exposes fixed 10-row pagination. The adapter verifies every declared page, unique Eightfold and Microsoft job IDs, matching detail records, official URLs, dates, descriptions, and explicit Netherlands locations before accepting a complete scan.
- Live discovery: the API declared 26 search results on 2026-08-14. All 26 details matched; 24 had explicit Netherlands locations and two Brussels-only search false positives were excluded. The count is volatile and is not hardcoded in the smoke test.
- Microsoft Careers reports 220,000 employees: <https://careers.microsoft.com/v2/global/en/locations/amsterdam.html>. The configured `2,000+` band follows the app's EU large-company classification.
- The configured industries follow Microsoft's own description of its software, cloud, AI, devices, gaming, and advertising businesses: <https://www.microsoft.com/investor/reports/ar25/index.html>.
- The careers API is English, but language and work-authorisation requirements vary by vacancy. Microsoft's university FAQ says visa sponsorship exists, while its Netherlands internship rules require the right to work for some programmes: <https://careers.microsoft.com/v2/global/en/universityinternship> and <https://careers.microsoft.com/v2/global/en/internship_eligibility>. Confirm sponsorship on the specific role.
- The API identifies the Microsoft brand, not the legal contract entity for each vacancy. Confirm that entity before relying on recognised-sponsor status.

## Amazon / AWS

- Official source: <https://www.amazon.jobs/en/search?country=NLD>. Its JSON API at <https://www.amazon.jobs/en/search.json?normalized_country_code%5B%5D=NLD&offset=0&result_limit=100> declares the exact Netherlands total. The adapter follows every result page and validates unique feed IDs, public job IDs, official paths, Netherlands locations, dates, full descriptions, and Amazon-owned application URLs.
- Amazon reported 1,576,000 full-time and part-time employees at year-end 2025: <https://ir.aboutamazon.com/news-release/news-release-details/2026/Amazon-com-Announces-Fourth-Quarter-Results/>. The configured `2,000+` band follows the app's EU large-company classification.
- The Netherlands board covers Amazon, AWS, advertising, media, logistics, devices, and other Amazon group employers. The configured industries describe that mixed board rather than every individual legal entity.
- Language and relocation conditions vary by vacancy. The API exposes the hiring company name, but it does not prove visa sponsorship; confirm the contract entity and sponsorship before relying on recognised-sponsor status.

## Uber

- Official source: <https://jobs.uber.com/en/jobs/?location=Amsterdam&radius=100>. Uber's official site links to the `UberCareers` Oracle HCM tenant at <https://iaziqy.fa.ocs.oraclecloud.com/hcmUI/CandidateExperience/en/sites/UberCareers/>. The adapter uses Oracle's documented candidate-experience requisition API at <https://docs.oracle.com/en/cloud/saas/human-resources/farws/op-recruitingcejobrequisitionscoordinates-get.html>, follows the declared total and pagination, requires unique numeric IDs, fetches every detail, and accepts only roles whose primary or secondary locations explicitly include country code `NL`.
- Uber's 2025 Form 10-K reports approximately 34,000 employees globally and describes Mobility, Delivery, Freight, technology, advertising, and payments activities: <https://d18rn0p25nwr6d.cloudfront.net/CIK-0001543151/22e9f27b-deea-485f-b5b6-7ecc7462b84a.pdf>. The configured `2,000+` band follows the app's EU large-company classification.
- The official Netherlands results and details are in English, but language, relocation, work-authorisation, and visa sponsorship conditions are role-specific. Confirm them on each vacancy; the source does not prove a company-wide sponsorship promise.
- The board identifies the Uber brand, while contracts may use different Uber group entities. Confirm the legal employer before relying on recognised-sponsor status.

## PostNL

- Official source: <https://vacatures-website.postnl.nl/vacatures-widget/api/vacanciesoverview?isProfessional=true&distance=-1&page=1>. The adapter follows every declared page, verifies the exact total and unique IDs, fetches every official detail, and accepts only professional roles with matching Netherlands-widget locations and official job/application URLs.
- PostNL reported 31,531 employees at year-end 2025: <https://annualreport.postnl.nl/2025/sustainability-statements/3-social-disclosures/3-2-own-workforce>. The configured `2,000+` band follows the app's EU large-company classification.
- Language requirements vary. The current professional feed includes English-language technology roles as well as roles requiring Dutch; check each vacancy.
- The API identifies the PostNL brand, not the legal contract entity for every role. Confirm that entity and visa support before relying on recognised-sponsor status.

## Achmea

- Official source: <https://www.werkenbijachmea.nl/vacatures>. Every page publishes an exact result range and stable total; the adapter verifies all pages, unique official vacancy IDs, matching JobPosting details, official application links, and explicit Netherlands evidence before accepting the scan.
- Achmea reports more than 18,000 colleagues, including 14,000 in the Netherlands: <https://www.achmea.nl/en/organisation>. The configured `2,000+` band follows the app's EU large-company classification.
- The board covers Achmea and its insurance and financial-services brands. Language and sponsorship conditions vary by vacancy; check each vacancy before applying.
- Job details identify the Achmea brand, not necessarily the legal contract entity. Confirm that entity before relying on recognised-sponsor status.

## ANWB

- Official source: <https://www.werkenbijanwb.nl/fuse/vacancies.json>. The one-response feed exposes all public vacancy IDs and official URLs; the adapter validates every feed row against its JobPosting detail, Netherlands location, publication date, hiring brand, description, and on-page application form.
- ANWB reports more than 4,000 colleagues: <https://www.werkenbijanwb.nl/over-ons>. The configured `2,000+` band follows the app's EU large-company classification.
- ANWB's IT page and current technical vacancies state that Dutch is required for some roles, including C1 Dutch for some positions. Check each vacancy before applying.
- The board covers several ANWB group hiring brands, including ANWB Energie and Unigarant. Confirm the legal contract entity before relying on recognised-sponsor status.

## ChipSoft

- Official source: <https://www.chipsoft.com/nl-NL/werken-bij/vacatures>. The server-rendered board exposes every vacancy in one response without pagination. The adapter validates unique official links, an explicit NL location allowlist, matching detail fields, and unique official application-form IDs; Antwerp vacancies are excluded.
- ChipSoft reports more than 1,000 professionals: <https://www.chipsoft.com/nl-nl/organisatie/over-ons/>. The configured `1,000+` band is approximate.
- Current developer roles require Dutch, including the .NET/C# roles. ChipSoft is therefore lower priority for applicants who do not speak Dutch.
- The board identifies the ChipSoft brand, not the legal contract entity for every role. Confirm that entity before relying on recognised-sponsor status.

## NS

- Official source: <https://www.werkenbijns.nl/vacatures>. Every page publishes an exact result range and stable total; the adapter verifies all pages, unique official vacancy IDs, matching JobPosting details, official application links, and explicit Netherlands evidence before accepting the scan.
- NS reports more than 20,000 colleagues: <https://www.ns.nl/en/about-ns>. The configured `2,000+` band follows the app's EU large-company classification.
- Current technical roles include .NET/C# work, but language and sponsorship conditions vary by vacancy. Check the vacancy before applying.
- The board identifies the NS brand, not the legal contract entity for every role. Confirm that entity before relying on recognised-sponsor status.

## AFAS Software

- Official source: <https://www.werkenbijafas.nl/alle-vacatures>. The board declares a complete one-page result with a 75-vacancy capacity; the adapter verifies that state, deduplicates stable official job links, reads each JobPosting detail, and keeps only explicit Netherlands locations.
- Current vacancy pages report approximately 650 colleagues. The configured `500–999` band is approximate and should be rechecked when AFAS publishes a newer company-wide figure.
- Current software vacancies require Dutch. AFAS is therefore lower priority for applicants who do not speak Dutch, regardless of residence status.
- The official details identify AFAS Software B.V.; confirm the employment entity and sponsor conditions on each contract before relying on sponsorship.

## Exact

- Official source: <https://www.exact.com/careers/vacancies>. The board exposes complete 20-item pagination, explicit country labels, stable vacancy IDs, and JobPosting JSON-LD details.
- Exact's official company profile reports more than 2,000 employees and 675,000 customers: <https://www.exact.com/about-us>.
- Sponsorship varies by vacancy; some current roles explicitly reject applicants who need visa sponsorship. Check each vacancy before applying.

## Bitvavo

- Official source: <https://api.ashbyhq.com/posting-api/job-board/bitvavo>. Bitvavo's official careers site labels the same roles as Amsterdam while Ashby calls the location `Headquarters`: <https://jobs.bitvavo.com/find-your-role>.
- Bitvavo states that all employees are based in Amsterdam and that eligible international hires receive relocation support: <https://jobs.bitvavo.com/life-at-bitvavo>.
- Bitvavo's official company page reports 500+ employees: <https://bitvavo.com/en/about>.
- The official careers footer identifies Bitvavo B.V.; confirm the contract entity on the vacancy before relying on sponsor status.

## Vanderlande

- Official source: <https://vanderlande.wd3.myworkdayjobs.com/careers>. The Workday API exposes an explicit Netherlands facet, a first-page total, paginated stable paths, and complete detail records.
- Vanderlande's official facts page reports more than 9,000 employees: <https://www.vanderlande.com/about-vanderlande/facts-and-figures/>.
- Workday exposes vacancy details but not a separately validated contract entity for every role. Confirm that entity before relying on sponsor status.

## Wolters Kluwer

- Official source: <https://wk.wd3.myworkdayjobs.com/External>. The Workday API exposes an explicit Netherlands facet, a first-page total, paginated stable paths, and complete detail records.
- Wolters Kluwer's official company profile reports 21,100 employees worldwide: <https://www.wolterskluwer.com/en/about-us>.
- Workday exposes the hiring organisation for each vacancy. Confirm that entity before relying on sponsor status.

## Info Support

- Official source: <https://werkenbij.infosupport.com/en/jobs.json>. The Teamtailor JSON feed exposes every public job, stable identifiers, structured locations, descriptions, and publication dates in one response.
- Current job descriptions report approximately 500 Info Support employees.
- The feed identifies Info Support as the hiring organisation. Confirm the contract entity before relying on sponsor status.

## Keylane

- Official source: <https://apply.workable.com/keylane/>. Its cursor-paginated Workable API exposes the complete public board and job details; the configured filter keeps only jobs explicitly located in the Netherlands.
- Keylane's official careers site reports 1,100+ employees: <https://careers.keylane.com/about-us/>.
- The board identifies the Keylane brand, not the legal employer for every contract. Confirm the employment entity before relying on sponsor status.

## Finom

- Official source: <https://api.eu.lever.co/v0/postings/pnlfin?mode=json>. Finom's official careers page links the `pnlfin` Lever tenant. The endpoint returns the complete public board in one response; the configured country filter keeps only explicit Amsterdam or Netherlands postings.
- Finom's official careers page reports 500+ employees: <https://careers.finom.co/>.
- The board identifies the Finom brand, not the legal employer for each contract. Confirm the employment entity before relying on sponsor status.

## TomTom

- Official source: <https://www.tomtom.com/careers/joboverview/>. TomTom's first-party careers API at <https://www.tomtom.com/api/careers/jobs> and its Lever board at <https://api.eu.lever.co/v0/postings/tomtom?mode=json> currently expose the same complete 32-role board.
- The configured Netherlands filter includes explicit Amsterdam and multi-location roles that list Amsterdam as an eligible location. TomTom's first-party board currently has 14 Amsterdam-primary roles; Lever additionally identifies one Madrid-primary role as eligible in Amsterdam, Gent, or Madrid.
- TomTom describes a workforce of 3,300+ in its current job descriptions. The board does not state visa sponsorship per role; confirm it before relying on sponsor status.

## Silverflow and Ohpen

- Official sources: <https://silverflow.jobs.personio.com/xml?language=en> and <https://ohpen.jobs.personio.com/xml?language=en>. Each Personio XML feed returns every public position with stable IDs, offices, descriptions, employment metadata, and creation dates in one response.
- Silverflow's current feed states that its international team has 80+ colleagues. Ohpen's public band is approximate until a newer first-party headcount is published.
- The feeds identify company brands, not the legal employer for every contract. Confirm the employment entity before relying on sponsor status.

## Hosted ATS expansion

The following enabled companies use the same complete-board adapters documented elsewhere in this file. Every Greenhouse source is filtered to explicit Netherlands offices; Recruitee and Ashby scans consume their complete public board before the application filter runs.

- IMC Trading: <https://boards-api.greenhouse.io/v1/boards/imc/jobs?content=true>
- Flow Traders: <https://boards-api.greenhouse.io/v1/boards/flowtraders/jobs?content=true>
- Maven Securities: <https://boards-api.greenhouse.io/v1/boards/mavensecuritiesholdingltd/jobs?content=true>. Maven's official careers page embeds this exact board: <https://www.mavensecurities.com/jobs/>.
- bunq: <https://bunq.recruitee.com/api/offers/>
- DPG Media: <https://vacatures.dpgmedia.nl/api/offers/>
- Miro: <https://api.ashbyhq.com/posting-api/job-board/miro>
- Checkout.com: <https://api.ashbyhq.com/posting-api/job-board/checkout.com>
- Fourthline: <https://boards-api.greenhouse.io/v1/boards/fourthline/jobs?content=true>
- Ockto: <https://ockto.recruitee.com/api/offers/>
- DRW: <https://boards-api.greenhouse.io/v1/boards/drweng/jobs?content=true>
- Jump Trading: <https://boards-api.greenhouse.io/v1/boards/jumptrading/jobs?content=true>
- Tower Research: <https://boards-api.greenhouse.io/v1/boards/towerresearchcapital/jobs?content=true>
- WEBB Traders: <https://webbtraders.recruitee.com/api/offers/>
- STX Group: <https://boards-api.greenhouse.io/v1/boards/stxgroup/jobs?content=true>
- Elastic: <https://boards-api.greenhouse.io/v1/boards/elastic/jobs?content=true>
- MultiSafepay: <https://careers.multisafepay.com/api/offers/>
- ACT Commodities: <https://boards-api.greenhouse.io/v1/boards/testendouble/jobs?content=true>. The unusual board token is linked by ACT's official careers site.

Each board names the company brand, not necessarily the legal employer for every contract. Confirm the employment entity before relying on sponsor status. Scale bands are approximate public headcount bands and should be rechecked when companies publish newer first-party figures.

Maven's Greenhouse payload declares the complete global total; the configured country filter keeps only vacancies with an explicit Netherlands office. Live verification on 2026-08-14 returned 37 unique global jobs and 4 Amsterdam jobs with complete descriptions. Maven's official history reports 386 employees in 2023 and an Amsterdam office for 46 people, so the configured `200+` band is conservative: <https://www.mavensecurities.com/>. Its emerging-talent page states that graduate roles can offer visa sponsorship, but internship applicants need existing regional work rights; experienced roles do not make a general sponsorship promise: <https://www.mavensecurities.com/emerging-talent/>.

## Da Vinci

- Official source: <https://boards-api.greenhouse.io/v1/boards/davinciderivatives/jobs?content=true>. The Greenhouse payload declares its complete global total; the configured country filter keeps only vacancies with an explicit Netherlands office.
- Careers ownership: Da Vinci's official site describes the Amsterdam trading firm and links its careers flow: <https://davincitrading.com/about-us/>.
- Scale: Da Vinci's company-managed LinkedIn profile reports more than 200 employees; treat this as an approximate band rather than an audited headcount: <https://www.linkedin.com/company/da-vinci-trading>.
- Hiring entity: the board names the Da Vinci brand, not the legal employer for each contract. Confirm the employment entity before relying on any sponsor status.

## Backbase

- Official source: <https://boards-api.greenhouse.io/v1/boards/workatbackbase/jobs?content=true>. The Greenhouse payload declares its complete global total; the configured country filter keeps only vacancies with an explicit Netherlands office.
- Careers ownership: Backbase's official careers site publishes the same vacancies: <https://www.backbase.com/careers/jobs>.
- Scale: Backbase reports 2,000+ employees globally: <https://www.backbase.com/about>.
- Hiring entity: the board names the Backbase brand, not the legal employer for each contract. Confirm the employment entity before relying on any sponsor status.

## Reddit

- Sponsor: the IND public register for work dated 2026-08-03 lists `Reddit Netherlands B.V.` with KvK `83433880`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: the vacancies identify Reddit, not the Dutch legal employer or its KvK number. Confirm that `Reddit Netherlands B.V.` is the employment entity before relying on sponsor status.
- Official source: <https://boards-api.greenhouse.io/v1/boards/reddit/jobs?content=true>. The API's `meta.total` verifies the complete global payload; the configured country filter accepts only jobs with an explicit Netherlands office.
- Live discovery: 7 complete Netherlands jobs on 2026-08-14. IDs were unique and every accepted job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## Databricks

- Sponsor: the IND public register for work dated 2026-08-03 lists `Databricks` with KvK `51208121`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: the register omits a legal-form suffix and the vacancies identify only the Databricks brand. Confirm the exact Dutch employment entity and KvK number before relying on sponsor status.
- Official source: <https://boards-api.greenhouse.io/v1/boards/databricks/jobs?content=true>. The API's `meta.total` verifies the complete global payload. The configured country filter then accepts only jobs with an explicit Netherlands office, avoiding unsafe country guesses for regional labels such as `APAC` and `EMEA`.
- Live discovery: 18 complete Netherlands jobs on 2026-08-14. IDs were unique and every accepted job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## DataSnipper

- Sponsor: the IND public register for work dated 2026-08-03 lists `DataSnipper B.V.` with KvK `69343861`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>. DataSnipper's official terms identify the Amsterdam entity with the same legal name and KvK number: <https://www.datasnipper.com/pdf-proxy.pdf?url=https%3A%2F%2Feu-assets.contentstack.com%2Fv3%2Fassets%2Fbltc08aa646f32b9827%2Fblt2109238ccab969a1%2F696a12bc34206b2e8465af0d%2FDataSnipper_Terms_and_Conditions_-_Version_2025-07-01.pdf>.
- Hiring entity: the careers board identifies DataSnipper and Amsterdam vacancies, but does not prove that every eventual contract is with `DataSnipper B.V.`. Confirm the employment entity before relying on sponsor status.
- Official source: <https://api.ashbyhq.com/posting-api/job-board/datasnipper>. The existing Ashby adapter consumes this complete single-endpoint board and normalises its `The Netherlands` country label.
- Live discovery: 32 complete jobs on 2026-08-14, including Netherlands vacancies. IDs were unique and every job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## ABN AMRO

- Sponsor: the current IND public register for work, updated 2026-08-03, lists `ABN AMRO Bank N.V.` with KvK `34334259`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>. ABN AMRO's own disclaimer identifies `ABN AMRO Bank N.V.` with the same Chamber of Commerce number: <https://www.abnamro.com/research/en/home/information/disclaimer>.
- Hiring entity: each accepted vacancy names only `ABN AMRO` as its hiring organization. Mapping that brand to `ABN AMRO Bank N.V.` is an inference; confirm the employment entity before relying on sponsor status. The careers privacy statement covers ABN AMRO's recruitment process but does not establish the legal employer for every contract: <https://www.werkenbijabnamro.nl/en/privacy-statement>.
- Official source: <https://www.werkenbijabnamro.nl/api/vacancy/?pageNumber=1&sort=created&sortDir=DESC&filters%5BLand%5D%5B%5D=Nederland>. The API declares the exact total, current page, maximum records per page, and total page count. Every detail publishes a matching numeric vacancy ID, canonical URL, application endpoint, and full `JobPosting` JSON-LD. The adapter uses the numeric Getnoticed vacancy ID rather than the different, potentially non-unique ATS identifier in JSON-LD, and rejects pagination drift, gaps, early empty pages, count mismatches, duplicates, identity or URL mismatches, unresolved countries, and missing required detail fields.
- Source cache: listing and detail responses advertised `Cache-Control: public, s-maxage=86400` on 2026-08-13, so live discovery can lag source changes by up to 24 hours.
- Live discovery: 65 Netherlands jobs on 2026-08-13. All nine declared pages and details were fetched, IDs were unique, and every job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## ING

- Sponsor: the current IND public register for work, updated 2026-08-03, lists `ING Bank N.V.` with KvK `33031431`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>. ING's own articles of association identify `ING Bank N.V.` with the same Dutch trade-register number: <https://ing.com/binaries/content/assets/documents/files/articles_of_association_of_ing_bank_n.v.pdf>.
- Hiring entity: the careers site labels Netherlands vacancies `ING Bank`, and its applicant privacy statement identifies `ING Bank N.V.`, but neither establishes the legal employer for every eventual contract. Mapping a vacancy branded `ING Bank` to the registered sponsor is therefore an inference; confirm the employment entity before relying on sponsor status: <https://careers.ing.com/en/privacy-statement>.
- Official source: <https://careers.ing.com/en/location/netherlands-jobs/2618/2750405/2/en/search-jobs>. The listing publishes an exact total, page count, current page, and page size; each detail publishes matching `JobPosting` JSON-LD and an official apply URL. The adapter rejects pagination drift, count mismatches, duplicate IDs, listing/detail identity mismatches, unresolved countries, and missing required fields before accepting a complete scan.
- Live discovery: 58 Netherlands jobs on 2026-08-13. All declared pages and details were fetched, IDs were unique, and every job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## bol.com

- Sponsor: the current IND public register for work lists `Bol.com B.V.` with KvK `32147382`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>. Bol.com's own legal information identifies `bol.com B.V.` with the same KvK number: <https://lowlands.bol.com/pages/algemene-voorwaarden>.
- Hiring entity: bol.com's applicant privacy policy identifies `Bol. b.v.` as controller for applications made through the careers site, but it does not name the legal employer or publish its KvK number there. Treating a vacancy branded `bol` as employment by `Bol.com B.V.` is therefore an inference; confirm the employment entity before relying on sponsor status: <https://careers.bol.com/nl/privacywetgeving/>.
- Official source: <https://careers.bol.com/api/v1/jobs/>. The paginated API declares an exact total; the adapter rejects non-exact or changing totals, incomplete pages, duplicate IDs, non-public records, unresolved offices, and missing required fields before accepting a complete scan.
- Live discovery: 79 jobs on 2026-08-12. IDs were unique, every job had the parser-required fields, and every generated official detail URL returned success. The count is volatile and is not hardcoded in the smoke test.

## Funda

- Sponsor: the current IND public register for work lists `Funda Real Estate B.V.` with KvK `34242436`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>. Funda's own applicant privacy statement identifies Funda as the same legal entity and KvK number: <https://www.funda.nl/en/voorwaarden-en-beleid/privacyverklaring/sollicitant/>.
- Hiring entity: the applicant privacy statement applies to Funda vacancies and the application process, but does not prove the legal employer for every eventual contract. Confirm that the vacancy's employment entity is `Funda Real Estate B.V.` before relying on sponsor status.
- Official source: <https://jobs.funda.nl/api/offers/>. The public Recruitee endpoint returns the board in one `offers` array; the adapter rejects a missing array, duplicate IDs, or any offer without its required fields before accepting a complete scan.
- Live discovery: 8 jobs on 2026-08-12. IDs were unique and every job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## Adyen

- Sponsor: the IND public register for work lists `Adyen N.V.` with KvK `34259528`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>. Adyen's own legal information confirms the same entity and Dutch company number: <https://www.adyen.com/en_GB/licenses/europe>.
- Hiring entity: treating a Netherlands vacancy branded `Adyen` as employment by `Adyen N.V.` is an inference. The Greenhouse payload names only `Adyen`, not the legal employer or its KvK number; sponsor status must therefore be checked against the vacancy's eventual employment entity.
- Official source: <https://boards-api.greenhouse.io/v1/boards/adyen/jobs?content=true>. Its `meta.total` declares completeness for the single returned `jobs` array, which the adapter verifies before accepting a scan.
- Live discovery: 218 jobs on 2026-08-12. IDs were unique and every job had the parser-required fields. The count is volatile and is not hardcoded in the smoke test.

## Rabobank

- Sponsor: Rabobank identifies the brand as a trade name of `Coöperatieve Rabobank U.A.` with KvK `30046259`: <https://www.rabobank.com/conditions>. The current IND work register lists the same entity and KvK number: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: vacancies identify only `Rabobank`, not the employment-contract entity or KvK number. Mapping a Netherlands vacancy to `Coöperatieve Rabobank U.A.` is a strong inference, not vacancy-level proof; confirm the contract entity before relying on sponsor status.
- Official source: <https://rabobank.jobs/api/v1/jobs/> plus <https://rabobank.jobs/api/sitemap/>. The adapter paginates the complete global API, requires an exact stable total and unique IDs, matches every API ID to its canonical Dutch sitemap URL, then keeps Netherlands vacancies.
- Live discovery: 145 complete global jobs and 81 Netherlands jobs on 2026-08-14. Counts are volatile and are not hardcoded in the smoke test.

## Airwallex

- Sponsor: the IND public register of recognised sponsors for work dated 2026-08-03 lists `Airwallex (Netherlands) B.V.` with KvK `77519256`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: treating a Netherlands vacancy as hired by `Airwallex (Netherlands) B.V.` is an inference. Airwallex's public Ashby vacancies identify Airwallex and their locations, but do not name that legal entity or its KvK number on each vacancy.
- Official source: <https://api.ashbyhq.com/posting-api/job-board/airwallex>. The existing Ashby adapter consumes this complete single-endpoint board.
- Live discovery: 622 listed jobs on 2026-08-12, including 19 with a primary Netherlands location and 6 additional jobs with a secondary Netherlands location. IDs were unique and no listed job was missing a parser-required field. Counts are volatile and are not hardcoded in the smoke test.

## eBay

- Sponsor: the IND public register of recognised sponsors for work dated 2026-08-03 lists `eBay International Management B.V.` with KvK `71993312`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: eBay's Workday data names hiring organisation `0039 eBay Intl Management BV`. Mapping that abbreviated name to the registered entity above is an inference; the source does not publish the KvK number on each vacancy.
- Official source: <https://jobs.ebayinc.com/us/en/jobs-in-netherlands>. Its embedded `phApp.ddo.eagerLoadRefineSearch` publishes `totalHits` and listing jobs; `?from=10&s=1` supplies later pages. Each official detail page publishes matching `JobPosting` JSON-LD with the complete description and Netherlands location.
- Live discovery: 7 Netherlands jobs on 2026-08-12. This count is volatile; scans trust the source's internally consistent total, not the baseline.

## Eneco

- Sponsor candidates: the current IND public register lists `Eneco Zakelijk` (KvK `24296168`), `Eneco Diamond Hydrogen B.V.` (KvK `90411285`), and `N.V. Eneco` (KvK `24246970`): <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Official source: <https://www.werkenbijeneco.nl/vacatures?f=1270>. The adapter walks every Tech listing page, requires the displayed total and ranges to remain complete, rejects duplicate IDs, and parses each official detail's `JobPosting` JSON-LD.
- Live discovery: 11 complete Tech jobs on 2026-08-14. IDs were unique and every job had the required title, description, date, and Netherlands location. The count is volatile and is not hardcoded in the smoke test.
- Privacy evidence: Eneco's privacy controller, `Eneco B.V.`, is not employment proof and must not be used to infer the hiring entity.
- Hiring entity: each vacancy identifies only `Eneco`. The exact employment-contract entity remains unproven; confirm it before relying on sponsor status.

## Albert Heijn Tech

- Sponsor candidates: the current IND public register for work lists `Albert Heijn B.V.` with KvK `35012085` and `Albert Heijn Support B.V.` with KvK `34305784`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Official source: <https://werk.ah.nl/api/vacancy/> filtered to Hoofdkantoor and IT/Data-science, plus each public vacancy detail's `JobPosting` JSON-LD. The adapter uses the endpoint's required `X-Requested-With` request header, validates all pagination metadata and filters, rejects duplicate IDs, and requires matching official detail data.
- Live discovery: 11 complete Tech/Data-science jobs on 2026-08-14. IDs were unique and every job had the required title, description, date, and Netherlands location. The count is volatile and is not hardcoded in the smoke test.
- Privacy evidence: the recruitment privacy notice spans four entities. It does not establish which legal entity employs a vacancy candidate.
- Hiring entity: each vacancy identifies only `Albert Heijn`. The exact employment-contract entity remains unproven; confirm it before relying on sponsor status.
