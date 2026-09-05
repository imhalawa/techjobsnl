namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Represents the closed set of raw official vacancy-source configurations.</summary>
public abstract record SourceConfiguration
{
    private SourceConfiguration()
    {
    }

    /// <summary>Gets a user-facing source strategy name.</summary>
    public abstract string StrategyName { get; }

    /// <summary>Gets the source's primary configured reference.</summary>
    public abstract string Reference { get; }

    /// <summary>Configures an Ashby board.</summary>
    public sealed record Ashby : SourceConfiguration { public Ashby(string board) { Board = board; } public string Board { get; } public override string StrategyName => "Ashby"; public override string Reference => Board; }
    /// <summary>Configures a Greenhouse board.</summary>
    public sealed record Greenhouse : SourceConfiguration { public Greenhouse(string board, string? countryFilter) { Board = board; CountryFilter = countryFilter; } public string Board { get; } public string? CountryFilter { get; } public override string StrategyName => "Greenhouse"; public override string Reference => Board; }
    /// <summary>Configures a Jibe board.</summary>
    public sealed record Jibe : SourceConfiguration { public Jibe(string baseUrl, string client) { BaseUrl = baseUrl; Client = client; } public string BaseUrl { get; } public string Client { get; } public override string StrategyName => "Jibe"; public override string Reference => BaseUrl; }
    /// <summary>Configures an eBay listing.</summary>
    public sealed record Ebay : SourceConfiguration { public Ebay(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "eBay"; public override string Reference => ListingUrl; }
    /// <summary>Configures a Recruitee board.</summary>
    public sealed record Recruitee : SourceConfiguration { public Recruitee(string baseUrl) { BaseUrl = baseUrl; } public string BaseUrl { get; } public override string StrategyName => "Recruitee"; public override string Reference => BaseUrl; }
    /// <summary>Configures a Personio board.</summary>
    public sealed record Personio : SourceConfiguration { public Personio(string baseUrl) { BaseUrl = baseUrl; } public string BaseUrl { get; } public override string StrategyName => "Personio XML Feed"; public override string Reference => BaseUrl; }
    /// <summary>Configures a Lever API.</summary>
    public sealed record Lever : SourceConfiguration { public Lever(string apiUrl, string? countryFilter) { ApiUrl = apiUrl; CountryFilter = countryFilter; } public string ApiUrl { get; } public string? CountryFilter { get; } public override string StrategyName => "Lever"; public override string Reference => ApiUrl; }
    /// <summary>Configures a Workable account.</summary>
    public sealed record Workable : SourceConfiguration { public Workable(string account, string? countryFilter) { Account = account; CountryFilter = countryFilter; } public string Account { get; } public string? CountryFilter { get; } public override string StrategyName => "Workable"; public override string Reference => Account; }
    /// <summary>Configures a Workday board.</summary>
    public sealed record Workday : SourceConfiguration { public Workday(string baseUrl, string tenant, string site, string country, string countryCode) { BaseUrl = baseUrl; Tenant = tenant; Site = site; Country = country; CountryCode = countryCode; } public string BaseUrl { get; } public string Tenant { get; } public string Site { get; } public string Country { get; } public string CountryCode { get; } public override string StrategyName => "Workday"; public override string Reference => BaseUrl; }
    /// <summary>Configures Yuki's JSON feed.</summary>
    public sealed record Yuki : SourceConfiguration { public Yuki(string feedUrl) { FeedUrl = feedUrl; } public string FeedUrl { get; } public override string StrategyName => "Teamtailor JSON Feed"; public override string Reference => FeedUrl; }
    /// <summary>Configures a Teamtailor feed.</summary>
    public sealed record Teamtailor : SourceConfiguration { public Teamtailor(string feedUrl, string employer) { FeedUrl = feedUrl; Employer = employer; } public string FeedUrl { get; } public string Employer { get; } public override string StrategyName => "Teamtailor JSON Feed"; public override string Reference => FeedUrl; }
    /// <summary>Configures bol.com.</summary>
    public sealed record Bol : SourceConfiguration { public Bol(string baseUrl) { BaseUrl = baseUrl; } public string BaseUrl { get; } public override string StrategyName => "bol.com"; public override string Reference => BaseUrl; }
    /// <summary>Configures Coolblue.</summary>
    public sealed record Coolblue : SourceConfiguration { public Coolblue(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "Coolblue HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures PAY.</summary>
    public sealed record Pay : SourceConfiguration { public Pay(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "PAY. HubSpot HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures Buckaroo.</summary>
    public sealed record Buckaroo : SourceConfiguration { public Buckaroo(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "Buckaroo HTML + sitemap"; public override string Reference => ListingUrl; }
    /// <summary>Configures Rabobank.</summary>
    public sealed record Rabobank : SourceConfiguration { public Rabobank(string baseUrl, string country) { BaseUrl = baseUrl; Country = country; } public string BaseUrl { get; } public string Country { get; } public override string StrategyName => "Rabobank API"; public override string Reference => BaseUrl; }
    /// <summary>Configures Eneco.</summary>
    public sealed record Eneco : SourceConfiguration { public Eneco(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "Eneco HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures Exact.</summary>
    public sealed record Exact : SourceConfiguration { public Exact(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "Exact HTML + JSON-LD"; public override string Reference => ListingUrl; }
    /// <summary>Configures AFAS.</summary>
    public sealed record Afas : SourceConfiguration { public Afas(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "AFAS HTML + JSON-LD"; public override string Reference => ListingUrl; }
    /// <summary>Configures NS.</summary>
    public sealed record Ns : SourceConfiguration { public Ns(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "NS paged HTML + JSON-LD"; public override string Reference => ListingUrl; }
    /// <summary>Configures Achmea.</summary>
    public sealed record Achmea : SourceConfiguration { public Achmea(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "Achmea paged HTML + JSON-LD"; public override string Reference => ListingUrl; }
    /// <summary>Configures ChipSoft.</summary>
    public sealed record Chipsoft : SourceConfiguration { public Chipsoft(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "ChipSoft HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures ANWB.</summary>
    public sealed record Anwb : SourceConfiguration { public Anwb(string feedUrl) { FeedUrl = feedUrl; } public string FeedUrl { get; } public override string StrategyName => "ANWB JSON + JSON-LD"; public override string Reference => FeedUrl; }
    /// <summary>Configures PostNL.</summary>
    public sealed record Postnl : SourceConfiguration { public Postnl(string apiUrl) { ApiUrl = apiUrl; } public string ApiUrl { get; } public override string StrategyName => "PostNL paged API"; public override string Reference => ApiUrl; }
    /// <summary>Configures PGGM.</summary>
    public sealed record Pggm : SourceConfiguration { public Pggm(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "PGGM paged HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures Amazon.</summary>
    public sealed record Amazon : SourceConfiguration { public Amazon(string searchUrl) { SearchUrl = searchUrl; } public string SearchUrl { get; } public override string StrategyName => "Amazon Jobs API"; public override string Reference => SearchUrl; }
    /// <summary>Configures Uber.</summary>
    public sealed record Uber : SourceConfiguration { public Uber(string apiUrl) { ApiUrl = apiUrl; } public string ApiUrl { get; } public override string StrategyName => "Uber Oracle HCM API"; public override string Reference => ApiUrl; }
    /// <summary>Configures Microsoft.</summary>
    public sealed record Microsoft : SourceConfiguration { public Microsoft(string searchUrl) { SearchUrl = searchUrl; } public string SearchUrl { get; } public override string StrategyName => "Microsoft Careers API"; public override string Reference => SearchUrl; }
    /// <summary>Configures Deel.</summary>
    public sealed record Deel : SourceConfiguration { public Deel(string boardUrl) { BoardUrl = boardUrl; } public string BoardUrl { get; } public override string StrategyName => "Deel Jobs"; public override string Reference => BoardUrl; }
    /// <summary>Configures SuccessFactors HTML.</summary>
    public sealed record Successfactors : SourceConfiguration { public Successfactors(string listingUrl, string employer) { ListingUrl = listingUrl; Employer = employer; } public string ListingUrl { get; } public string Employer { get; } public override string StrategyName => "SAP SuccessFactors HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures Google.</summary>
    public sealed record Google : SourceConfiguration { public Google(string searchUrl) { SearchUrl = searchUrl; } public string SearchUrl { get; } public override string StrategyName => "Google Careers HTML"; public override string Reference => SearchUrl; }
    /// <summary>Configures the SuccessFactors API.</summary>
    public sealed record SuccessfactorsApi : SourceConfiguration { public SuccessfactorsApi(string baseUrl) { BaseUrl = baseUrl; } public string BaseUrl { get; } public override string StrategyName => "SAP SuccessFactors API"; public override string Reference => BaseUrl; }
    /// <summary>Configures Albert Heijn.</summary>
    public sealed record AlbertHeijn : SourceConfiguration { public AlbertHeijn(string baseUrl) { BaseUrl = baseUrl; } public string BaseUrl { get; } public override string StrategyName => "Albert Heijn API"; public override string Reference => BaseUrl; }
    /// <summary>Configures ING.</summary>
    public sealed record Ing : SourceConfiguration { public Ing(string listingUrl) { ListingUrl = listingUrl; } public string ListingUrl { get; } public override string StrategyName => "ING HTML"; public override string Reference => ListingUrl; }
    /// <summary>Configures Getnoticed.</summary>
    public sealed record Getnoticed : SourceConfiguration { public Getnoticed(string baseUrl, string? countryFilter) { BaseUrl = baseUrl; CountryFilter = countryFilter; } public string BaseUrl { get; } public string? CountryFilter { get; } public override string StrategyName => "Getnoticed"; public override string Reference => BaseUrl; }
    /// <summary>Retains the compatibility-only paged HTML model.</summary>
    public sealed record PagedHtml : SourceConfiguration { public PagedHtml(string listingUrl, string offsetParameter, int pageSize) { ListingUrl = listingUrl; OffsetParameter = offsetParameter; PageSize = pageSize; } public string ListingUrl { get; } public string OffsetParameter { get; } public int PageSize { get; } public override string StrategyName => "Paged HTML"; public override string Reference => ListingUrl; }
    /// <summary>Retains an explicitly disabled unsupported source.</summary>
    public sealed record Unsupported : SourceConfiguration { public Unsupported(string reason) { Reason = reason; } public string Reason { get; } public override string StrategyName => "Unsupported"; public override string Reference => Reason; }
}
