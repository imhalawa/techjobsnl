# Source and sponsor evidence

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
