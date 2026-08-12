# Source and sponsor evidence

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

- Sponsor: the IND public register of recognised sponsors for work dated 2026-07-01 lists `Airwallex (Netherlands) B.V.` with KvK `77519256`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: treating a Netherlands vacancy as hired by `Airwallex (Netherlands) B.V.` is an inference. Airwallex's public Ashby vacancies identify Airwallex and their locations, but do not name that legal entity or its KvK number on each vacancy.
- Official source: <https://api.ashbyhq.com/posting-api/job-board/airwallex>. The existing Ashby adapter consumes this complete single-endpoint board.
- Live discovery: 622 listed jobs on 2026-08-12, including 19 with a primary Netherlands location and 6 additional jobs with a secondary Netherlands location. IDs were unique and no listed job was missing a parser-required field. Counts are volatile and are not hardcoded in the smoke test.

## eBay

- Sponsor: the IND public register of recognised sponsors for work dated 2026-07-01 lists `eBay International Management B.V.` with KvK `71993312`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: eBay's Workday data names hiring organisation `0039 eBay Intl Management BV`. Mapping that abbreviated name to the registered entity above is an inference; the source does not publish the KvK number on each vacancy.
- Official source: <https://jobs.ebayinc.com/us/en/jobs-in-netherlands>. Its embedded `phApp.ddo.eagerLoadRefineSearch` publishes `totalHits` and listing jobs; `?from=10&s=1` supplies later pages. Each official detail page publishes matching `JobPosting` JSON-LD with the complete description and Netherlands location.
- Live discovery: 7 Netherlands jobs on 2026-08-12. This count is volatile; scans trust the source's internally consistent total, not the baseline.

## Eneco

- Sponsor candidates: the current IND public register lists `Eneco Zakelijk` (KvK `24296168`), `Eneco Diamond Hydrogen B.V.` (KvK `90411285`), and `N.V. Eneco` (KvK `24246970`): <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Official source: Eneco's public careers source returned 21 live postings on 2026-08-12. Each posting's `hiringOrganization` was only `Eneco`; it did not identify a legal employer or a KvK number that could be matched to a sponsor candidate.
- Privacy evidence: Eneco's privacy controller, `Eneco B.V.`, is not employment proof and must not be used to infer the hiring entity.
- Policy: Eneco remains configured as disabled and unsupported because its legal employer is not established. Re-enable it only when an official vacancy or employment document names the legal entity and its KvK number matches a current IND-recognised sponsor.
