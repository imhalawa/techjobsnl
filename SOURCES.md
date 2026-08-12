# Source and sponsor evidence

## eBay

- Sponsor: the IND public register of recognised sponsors for work dated 2026-07-01 lists `eBay International Management B.V.` with KvK `71993312`: <https://ind.nl/en/public-register-recognised-sponsors/public-register-work>.
- Hiring entity: eBay's Workday data names hiring organisation `0039 eBay Intl Management BV`. Mapping that abbreviated name to the registered entity above is an inference; the source does not publish the KvK number on each vacancy.
- Official source: <https://jobs.ebayinc.com/us/en/jobs-in-netherlands>. Its embedded `phApp.ddo.eagerLoadRefineSearch` publishes `totalHits` and listing jobs; `?from=10&s=1` supplies later pages. Each official detail page publishes matching `JobPosting` JSON-LD with the complete description and Netherlands location.
- Live discovery: 7 Netherlands jobs on 2026-08-12. This count is volatile; scans trust the source's internally consistent total, not the baseline.
