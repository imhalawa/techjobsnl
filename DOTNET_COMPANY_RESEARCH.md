# Netherlands .NET company research

Checked **14 August 2026**. TechPays was used only for discovery and market relevance. Current-role, technology, sponsor, language, and source-readiness claims below come from first-party career sites and the official IND register.

## Scale and industry fit

Scale is measured from current first-party company figures, not job-board size. Industry fit means similarity to the tracker's existing fintech, banking, marketplace, travel, e-commerce, and B2B SaaS companies. Parent-company scale is kept separate from the specific Dutch employer because a large parent does not prove that a local team is large or offers sponsorship.

| Company | Verified scale | Industry | Fit with current tracker | Effect on priority |
|---|---|---|---|---|
| Coolblue | **€2.563 billion 2025 revenue**, operating across the Netherlands, Belgium, and Germany. | Consumer technology, e-commerce, retail, delivery, and energy. | **Very high:** closest to bol.com, Albert Heijn, eBay, and Booking.com in product scale and consumer traffic. | Confirms **rank 1**. It combines large-scale product engineering with the cleanest source. |
| Topicus | Parent Topicus.com reports **10,000+ employees**, **100,000+ customers**, **40 vertical markets**, and **€1.552 billion consolidated revenue**. The Dutch Topicus operating group's own size is not separately stated. | Vertical-market B2B software across finance, healthcare, education, government, and other regulated sectors. | **High:** close to the banks and DataSnipper in regulated software, while adding public-sector and healthcare products. | Remains **rank 2**. Parent scale is strong, but the decentralised structure and Dutch vacancy page reduce certainty for international applicants. |
| Exact | **2,000 colleagues** and **675,000 business customers**, mainly in the Netherlands, Belgium, and Germany. | Accounting, finance, HR, and ERP SaaS for SMEs. | **High:** directly adjacent to DataSnipper and the banking/fintech sources. | Scale and industry are strong, but they do not override the explicit no-sponsorship rule on both current .NET roles. |
| Visma / Yuki | Visma reported **€2.803 billion 2025 revenue** and **2.4 million customers**; the group has **170+ decentralised business units**. These are parent figures, not Yuki figures. | Mission-critical accounting, payroll, HR, and public-sector SaaS. | **High:** strong B2B finance-software fit. | Large enough, but keep on the watchlist: the currently indexed Rotterdam Yuki .NET vacancies show **31 July 2026 deadlines**, already past at the time of this check, and the source spans many independent employers. |
| CM.com | **600+ FTE** and **24 offices globally**. | Customer engagement, communications, AI, and payments. | **Very high:** close to Mollie, Adyen, and Airwallex. | Good scale and industry, but current .NET vacancies require existing permanent Dutch work rights. |
| AFAS | The current vacancy states **70+ developers** in a **150-person product-development organisation**. | HR, payroll, accounting, and ERP SaaS. | **High:** B2B software adjacent to Exact and DataSnipper. | Lower scale than the leaders; fluent Dutch and no verified IND sponsor remain hard blockers. |

## Recommended onboarding queue

| Rank | Company | Current NL .NET evidence | IND recognised sponsor | Source readiness | Decision |
|---:|---|---|---|---|---|
| 1 | Coolblue | **2 current C# roles** in Rotterdam: C# Developer and Team Lead C#. The developer vacancy specifies C#, .NET 8, .NET Web API/MVC, AWS, and English. | **Yes** — Coolblue B.V., KvK 24330087. | **High.** The official server-rendered search page returned HTTP 200 and all **13 English vacancies** in one response. Individual job pages are stable and complete. | **Onboard first.** Strong product-company fit, active .NET hiring, and the easiest complete source. |
| 2 | Topicus | **1 directly verified .NET Developer role** in Deventer (32–40 hours), using .NET 10, Angular 22, Blazor, with Azure preferred. Its official board currently reports **66 total vacancies** and also contains lead-developer vacancies mentioning .NET. | **Yes** — Topicus.com Coöperatief U.A., KvK 59421916; Topicus.Finance B.V. and Topicus.Healthcare B.V. are also listed. | **High.** The official Getnoticed board and detail page returned HTTP 200 with server-rendered data. The project already has a Getnoticed source implementation that should be reused. | **Onboard second.** The role page is Dutch, but it does **not** state Dutch as a requirement; treating it as English-friendly would be an inference, not evidence. |
| 3 | Exact | Its current official Technology board contains **2 permanent Netherlands .NET roles**: .NET Software Engineer Exact and Junior Software Engineer .NET. Exact describes a Microsoft-oriented stack: .NET Framework, C#/VB, ASP.NET, Azure SQL, REST APIs, and webhooks. | **Yes** — Exact Group B.V., KvK 27225828, and Exact Cloud Development Benelux B.V., KvK 61877107. | **Medium.** The official server-rendered Technology board returned HTTP 200 and exposes stable vacancy links, but mixes countries and requires strict Netherlands filtering. | **Defer for this tracker if sponsorship is required.** Both current Utrecht .NET listings explicitly say they do **not** accept applicants needing visa sponsorship. A previously indexed Delft/Eindhoven vacancy returned HTTP 404 today and was excluded. |

## Verified lower-priority candidates

| Company | Current evidence | Sponsor/language blocker | Source readiness | Decision |
|---|---|---|---|---|
| CM.com | **2 current official .NET vacancies**: Senior Full Stack Developer (C#, .NET, Entity Framework, SQL, Azure) and Principal .NET backend developer. | CM.com International B.V. and two CM.com R&D entities are recognised sponsors. However, both vacancies require an existing permanent Dutch work/residence permit; the principal role also requires Dutch. | **High.** `jobs.cm.com` is a Recruitee board; reuse the existing Recruitee implementation. Pages returned HTTP 200. | Do not onboard for visa-seeking users unless the scope includes roles that cannot sponsor. |
| AFAS | **1 current official Software Engineer role** using C#, .NET Core, and SQL; the page says the product organisation has over 70 developers. | **Fluent written and spoken Dutch is required.** No AFAS entity was found in the current IND work-sponsor register. | **High.** Official vacancy returned HTTP 200 with server-rendered content. | Exclude from the current queue. |

## Not ready

- **Nationale-Nederlanden:** its official technology page lists C# among a large technology organisation's stack, but no live first-party Netherlands .NET vacancy was verified today.
- **Just Eat Takeaway.com:** its official .NET vacancy is marked filled; no current .NET vacancy was verified.
- **Wolters Kluwer:** its official Netherlands careers presence and multiple recognised sponsor entities are verified, but no live first-party Netherlands .NET vacancy was verified today.
- **Visma/Yuki:** the official Visma board currently shows four Netherlands Yuki roles tagged C#/.NET, and The Yuki Company is an IND-recognised sponsor. This is promising, but it was not placed ahead of the queue because the board represents many independently operated Visma companies; onboarding it as one company would need an explicit product/entity decision.

## Primary sources

- TechPays discovery reference: [Netherlands engineering compensation](https://techpays.com/europe/netherlands/eng_web). TechPays currently blocks unattended access with Cloudflare, so it is **not** suitable as a job source.
- Official IND evidence: [Public register Work](https://ind.nl/en/public-register-recognised-sponsors/public-register-work), updated **3 August 2026**. A register match proves recognised-sponsor status for the named legal entity; it does **not** prove that every vacancy offers sponsorship.
- Coolblue: [C# Developer](https://www.coolblue.nl/en/vacancies/c-developer), [all vacancies](https://www.coolblue.nl/en/vacancies/search), [tech vacancies](https://www.coolblue.nl/en/vacancies/tech).
- Topicus: [.NET Developer](https://www.werkenbijtopicus.nl/vacature/271/net-developer-2), [official vacancy board](https://www.werkenbijtopicus.nl/vacatures/expertise/meer-mooie-vacatures).
- Exact: [Technology board and stack](https://www.exact.com/careers/teams/technology). The stale vacancy excluded after a live 404 check was `a0tSi00000L09iNIAR-net-software-engineer`.
- CM.com: [Senior Full Stack Developer](https://jobs.cm.com/o/senior-full-stack-developer-mobile-service-cloud), [Principal Developer](https://jobs.cm.com/o/principal-developer-mobile-service-cloud).
- AFAS: [Software Engineer](https://www.werkenbijafas.nl/job/software-engineer).
- Visma: [official open positions](https://www.visma.com/careers/open-positions).
- Scale: [Coolblue 2025 yearbook](https://aboutcoolblue.com/en/yearbook/a-year-in-review/), [Topicus.com about page](https://topicus.com/about-us), [Exact about page](https://www.exact.com/about-us?layout=default&print=1&tmpl=component), [Visma 2025 results](https://www.visma.com/newsroom/visma-delivers-strong-2025-with-record-customer-intake-and-continued-ai-led-innovation), and [CM.com investor relations](https://www.cm.com/investor-relations/).
- Wolters Kluwer: [Netherlands technology careers](https://careers.wolterskluwer.com/nl-nl/technology).

## Practical result

Implement **Coolblue → Topicus** now. Exact is technically relevant but its current .NET openings explicitly reject sponsorship needs. Recheck Exact, CM.com, and the watchlist during later expansion rather than adding sources that mostly produce ineligible jobs.
