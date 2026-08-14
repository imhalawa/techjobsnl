# Source and sponsor evidence

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
- Official source: Eneco's public careers source returned 21 live postings on 2026-08-12. Each posting's `hiringOrganization` was only `Eneco`; it did not identify a legal employer or a KvK number that could be matched to a sponsor candidate.
- Privacy evidence: Eneco's privacy controller, `Eneco B.V.`, is not employment proof and must not be used to infer the hiring entity.
- Policy: Eneco remains configured as disabled and unsupported because its legal employer is not established. Re-enable it only when an official vacancy or employment document names the legal entity and its KvK number matches a current IND-recognised sponsor.

## Albert Heijn Tech

- Sponsor candidates: the current IND public register for work lists `Albert Heijn B.V.` with KvK `35012085` and `Albert Heijn Support B.V.` with KvK `34305784`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Official source: the native official filtered source returned a technically complete set of 10 Tech/Data Science jobs on 2026-08-13. Every vacancy's `hiringOrganization` was only `Albert Heijn`.
- Privacy evidence: the recruitment privacy notice spans four entities. It does not establish which legal entity employs a vacancy candidate.
- Policy: Albert Heijn Tech remains configured as disabled and unsupported because the legal employer cannot be matched to an IND sponsor. Re-enable it only when an official vacancy, recruiter, or employment document names the legal entity and its KvK number matches a current IND-recognised sponsor.
