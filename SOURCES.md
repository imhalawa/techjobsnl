# Source and sponsor evidence

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

- Official source limitation: on 2026-08-12, normal-user-agent direct GETs to the official listing/search pages, page 2, a vacancy detail, `robots.txt`, sitemaps, WordPress REST API, and feeds all returned HTTP 403 with an AkamaiNetStorage-branded unavailable page. No complete official unattended source was available.
- Policy: Rabobank remains configured as disabled and unsupported. Re-enable it only when a complete official unattended source is accessible and passes source-contract fixtures plus live completeness verification; do not add browser impersonation or bypass the edge protection.

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
