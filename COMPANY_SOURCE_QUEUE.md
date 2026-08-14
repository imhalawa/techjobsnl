# Company source delivery queue

Audit date: **2026-08-14**. Scope: all **79 companies** from `CANDIDATE_SOURCE_COMPATIBILITY.md`.

One checkbox is one delivery task. A task is complete only after its source is implemented, tested, live-verified, and merged into local `main`. Blocked tasks must be rechecked against a complete first-party source; they must not be added as disabled placeholders.

## Priority 1–2: global technology, fintech, and trading

1. [x] **Backbase** — shipped · Greenhouse (`workatbackbase`).
2. [x] **Bitvavo** — shipped · Ashby with verified `Headquarters = NL` override.
3. [x] **Da Vinci Derivatives** — shipped · Greenhouse (`davinciderivatives`).
4. [ ] **Uber** — blocked/needs research · official Amsterdam custom careers search; no reusable adapter or complete feed proved.
5. [ ] **Optiver** — blocked/needs research · official custom careers site has Amsterdam technology roles; no complete supported feed proved.
6. [x] **IMC Trading** — shipped · Greenhouse (`imc`).
7. [x] **Flow Traders** — shipped · Greenhouse (`flowtraders`).
8. [ ] **Maven Securities** — blocked/needs research · official careers content exists; no stable complete first-party feed confirmed.
9. [ ] **Stripe** — blocked/needs research · Greenhouse is compatible, but the latest live check returned no Netherlands roles.
10. [x] **bunq** — shipped · Recruitee.
11. [x] **Finom** — shipped · Lever (`pnlfin`).
12. [x] **Silverflow** — shipped · Personio.
13. [x] **Elastic** — shipped · Greenhouse (`elastic`).
14. [x] **Miro** — shipped · Ashby.
15. [x] **TomTom** — shipped · Lever (`tomtom`); the official careers API exposes the same complete 32-role board.
16. [x] **Checkout.com** — shipped · Ashby.
17. [ ] **Wise** — blocked/needs research · no current NL role; main careers source is Attrax and the small Greenhouse board is not proved complete.
18. [ ] **Revolut** — blocked/needs research · official custom careers source had no Netherlands location and no reusable feed.
19. [ ] **Klarna** — blocked/needs research · live Amsterdam roles use unsupported Deel Jobs.
20. [x] **Fourthline** — shipped · Greenhouse (`fourthline`).
21. [ ] **flatexDEGIRO** — blocked/needs research · official custom careers site has Amsterdam roles; no complete supported endpoint proved.
22. [x] **Ohpen** — shipped · Personio.
23. [ ] **Worldline** — blocked/needs research · live NL roles use unsupported SAP SuccessFactors.
24. [ ] **Plaid** — blocked/needs research · no current NL role or supported complete source observed.
25. [ ] **Buckaroo** — blocked/needs research · first-party NL vacancy page exists; completeness and stable feed are unproved.
26. [ ] **PAY.** — blocked/needs research · first-party HubSpot-hosted careers page has NL roles; completeness semantics need proof.
27. [x] **MultiSafepay** — shipped · Recruitee.
28. [ ] **Knab** — blocked/needs research · current custom careers site has NL roles; former Greenhouse board returns 404.
29. [ ] **Brand New Day** — blocked/needs research · Amsterdam roles exist on a custom site; no stable complete feed proved.
30. [ ] **FRISS** — blocked/needs research · company says all openings are on LinkedIn; no complete first-party jobs feed.
31. [x] **Keylane** — shipped · Workable.
32. [ ] **Currence / iDEAL** — blocked/needs research · no current first-party vacancy board verified; ownership must be rechecked.
33. [ ] **Payaut** — blocked/needs research · official company presence exists, but no stable public jobs source was found.
34. [x] **Ockto** — shipped · Recruitee.
35. [x] **DRW** — shipped · Greenhouse (`drweng`).
36. [ ] **Jane Street** — blocked/needs research · complete Greenhouse board currently has no Netherlands role.
37. [x] **Jump Trading** — shipped · Greenhouse (`jumptrading`).
38. [x] **Tower Research** — shipped · Greenhouse (`towerresearchcapital`).
39. [x] **WEBB Traders** — shipped · Recruitee.

## Priority 3–4: remaining companies

40. [x] **ACT Commodities** — shipped · Greenhouse (`testendouble`), linked from the official careers flow.
41. [x] **STX Group** — shipped · Greenhouse (`stxgroup`).
42. [ ] **OTC Flow** — blocked/needs research · first-party careers links to unsupported BambooHR.
43. [ ] **Vitol** — blocked/needs research · complete SmartRecruiters feed had no Netherlands role.
44. [ ] **Amazon / AWS** — blocked/needs research · official proprietary platform has NL roles; no complete reusable feed proved.
45. [ ] **Google** — blocked/needs research · official proprietary search has Netherlands results; no complete reusable feed proved.
46. [ ] **Microsoft** — blocked/needs research · official proprietary platform has Amsterdam roles; no complete reusable feed proved.
47. [ ] **Meta** — blocked/needs research · public/login-gated interface did not safely expose a current Netherlands vacancy.
48. [ ] **PostNL** — ready/next · official API exposes 77 professional jobs across 8 pages with complete detail endpoints.
49. [ ] **PGGM** — blocked/needs research · official custom source has current .NET roles; no reusable complete feed proved.
50. [x] **NS** — shipped · NS custom/Hamilton feed.
51. [ ] **Achmea** — blocked/needs research · live roles use a Hamilton/custom feed; compatibility with the NS implementation needs proof.
52. [ ] **a.s.r.** — blocked/needs research · live roles use an unsupported custom Vue/API source.
53. [ ] **Nationale-Nederlanden** — blocked/needs research · live roles use an unsupported custom careers source.
54. [ ] **Alliander** — blocked/needs research · live roles use an unsupported custom Next.js/API source.
55. [x] **Exact** — shipped · Exact HTML + JSON-LD.
56. [x] **AFAS Software** — shipped · AFAS HTML + JSON-LD.
57. [x] **Wolters Kluwer** — shipped · Workday.
58. [x] **ChipSoft** — shipped · ChipSoft custom source.
59. [ ] **BNG Bank** — blocked/needs research · current vacancies use an unsupported custom recruitment API.
60. [ ] **Schiphol Group** — blocked/needs research · live roles use SAP SuccessFactors behind a custom front end.
61. [ ] **KLM** — blocked/needs research · Tech & Data roles are visible, but no stable complete feed was verified.
62. [x] **ANWB** — shipped · ANWB Fuse JSON feed.
63. [ ] **APG** — blocked/needs research · live roles use unsupported WillHire.
64. [ ] **UWV** — blocked/needs research · current NL-only careers page exists; no reusable complete feed identified.
65. [ ] **RDW** — blocked/needs research · live ICT roles use an unsupported custom faceted-search API.
66. [ ] **Kadaster** — blocked/needs research · vacancies exist, but the ATS and stable complete feed are not confidently identified.
67. [ ] **ProRail** — blocked/needs research · current roles use an unsupported custom careers source.
68. [ ] **IVO Rechtspraak** — blocked/needs research · vacancies exist, but the current IVO subset cannot yet be safely resolved.
69. [ ] **Defensie / JIVC** — blocked/needs research · COMMIT/JIVC roles use an unsupported custom careers source.
70. [ ] **Stedin** — blocked/needs research · IT/Data roles use unsupported Radancy/custom careers.
71. [ ] **TenneT** — blocked/needs research · live roles use unsupported Avature.
72. [ ] **Gasunie** — blocked/needs research · live roles use an unsupported custom vacancy API.
73. [ ] **KPN** — blocked/needs research · Tech & IT roles use an unsupported custom vacancy API.
74. [x] **DPG Media** — shipped · Recruitee.
75. [x] **Vanderlande** — shipped · Workday.
76. [ ] **Lely** — blocked/needs research · live roles use unsupported custom Optimizely/Episerver.
77. [ ] **Planon** — blocked/needs research · live NL roles use unsupported Talentsoft.
78. [ ] **ilionx** — blocked/needs research · official site is live, but no complete standard/public Teamtailor JSON feed was proved.
79. [x] **Info Support** — shipped · Teamtailor JSON feed.
