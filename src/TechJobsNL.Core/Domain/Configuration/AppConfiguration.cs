using System.Collections.Immutable;
using System.Text;
using System.Text.RegularExpressions;

namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Represents raw configuration data and validates it without environmental side effects.</summary>
public sealed record AppConfiguration
{
    private static readonly ImmutableHashSet<string> SupportedColours = ImmutableHashSet.Create(StringComparer.Ordinal,
        "reset", "black", "red", "green", "yellow", "blue", "magenta", "cyan", "gray", "darkgray", "lightred",
        "lightgreen", "lightyellow", "lightblue", "lightmagenta", "lightcyan", "white");

    /// <summary>Initializes a new instance of the <see cref="AppConfiguration"/> class.</summary>
    public AppConfiguration(int schemaVersion, string databasePath, ImmutableArray<CompanyConfiguration> companies,
        FiltersConfiguration filters, ScanConfiguration scan, AnalyticsConfiguration analytics, UiConfiguration ui,
        KeybindingsConfiguration keybindings)
    {
        SchemaVersion = schemaVersion;
        DatabasePath = databasePath;
        Companies = companies;
        Filters = filters;
        Scan = scan;
        Analytics = analytics;
        Ui = ui;
        Keybindings = keybindings;
    }

    public int SchemaVersion { get; }
    public string DatabasePath { get; }
    public ImmutableArray<CompanyConfiguration> Companies { get; }
    public FiltersConfiguration Filters { get; }
    public ScanConfiguration Scan { get; }
    public AnalyticsConfiguration Analytics { get; }
    public UiConfiguration Ui { get; }
    public KeybindingsConfiguration Keybindings { get; }

    /// <summary>Validates the configuration deterministically without reading or writing anything.</summary>
    public ConfigurationValidationResult Validate()
    {
        if (SchemaVersion != 1) return Invalid("schema_version", "must be exactly 1");
        if (Scan.Concurrency <= 0) return Invalid("scan.concurrency", "must be greater than zero");
        if (Scan.TimeoutSeconds <= 0) return Invalid("scan.timeout_seconds", "must be greater than zero");

        var filterResult = ValidateFilters();
        if (filterResult is not ConfigurationValidationResult.Valid) return filterResult;
        var analyticsResult = ValidateAnalytics();
        if (analyticsResult is not ConfigurationValidationResult.Valid) return analyticsResult;

        var companyIds = new HashSet<string>(StringComparer.Ordinal);
        for (var index = 0; index < Companies.Length; index++)
        {
            var company = Companies[index];
            if (string.IsNullOrWhiteSpace(company.Id)) return Invalid($"companies[{index}].id", "must not be empty");
            if (!companyIds.Add(company.Id)) return Invalid($"companies[{index}].id", "must be unique");
            foreach (var pair in company.LocationCountryOverrides)
            {
                var countryResult = ValidateCountry(pair.Value, $"companies[{index}].location_country_overrides.{pair.Key}");
                if (countryResult is not ConfigurationValidationResult.Valid) return countryResult;
            }

            var sourceResult = ValidateSource(company, index);
            if (sourceResult is not ConfigurationValidationResult.Valid) return sourceResult;
        }

        if (!string.Equals(Ui.Theme, "clean-dark", StringComparison.Ordinal) &&
            !string.Equals(Ui.Theme, "clean-light", StringComparison.Ordinal)) return Invalid("ui.theme", "must be one of: clean-dark, clean-light");
        var overridesResult = ValidateThemeOverrides();
        if (overridesResult is not ConfigurationValidationResult.Valid) return overridesResult;
        return ValidateKeybindings();
    }

    private ConfigurationValidationResult ValidateFilters()
    {
        if (Filters.NewJobMaxAgeDays <= 0) return Invalid("filters.new_job_max_age_days", "must be greater than zero");
        if (Filters.Countries.IsEmpty) return Invalid("filters.countries", "must contain at least one country");
        for (var index = 0; index < Filters.Countries.Length; index++)
        {
            var countryResult = ValidateCountry(Filters.Countries[index], $"filters.countries[{index}]");
            if (countryResult is not ConfigurationValidationResult.Valid) return countryResult;
        }

        return ValidatePatterns("include_title_patterns", Filters.IncludeTitlePatterns) is var includeResult &&
            includeResult is not ConfigurationValidationResult.Valid ? includeResult : ValidatePatterns("exclude_title_patterns", Filters.ExcludeTitlePatterns);
    }

    private ConfigurationValidationResult ValidateAnalytics()
    {
        if (Analytics.MinimumCooccurrence <= 0) return Invalid("analytics.minimum_cooccurrence", "must be greater than zero");
        if (Analytics.MinimumSkillOccurrence <= 0) return Invalid("analytics.minimum_skill_occurrence", "must be greater than zero");
        if (Analytics.MaximumSkills <= 0) return Invalid("analytics.maximum_skills", "must be greater than zero");
        if (Analytics.AiTimeoutSeconds <= 0) return Invalid("analytics.ai_timeout_seconds", "must be greater than zero");
        return new ConfigurationValidationResult.Valid();
    }

    private static ConfigurationValidationResult ValidatePatterns(string field, ImmutableArray<string> patterns)
    {
        for (var index = 0; index < patterns.Length; index++)
        {
            try { _ = new Regex(patterns[index], RegexOptions.IgnoreCase | RegexOptions.CultureInvariant, TimeSpan.FromSeconds(1)); }
            catch (ArgumentException error) { return Invalid($"filters.{field}[{index}]", $"must be a valid regular expression: {error.Message}"); }
        }

        return new ConfigurationValidationResult.Valid();
    }

    private ConfigurationValidationResult ValidateThemeOverrides()
    {
        foreach (var (token, value) in new[]
        {
            ("background", Ui.ThemeOverrides.Background), ("focused_border", Ui.ThemeOverrides.FocusedBorder),
            ("unfocused_border", Ui.ThemeOverrides.UnfocusedBorder), ("selected_row", Ui.ThemeOverrides.SelectedRow),
            ("primary_text", Ui.ThemeOverrides.PrimaryText), ("muted_text", Ui.ThemeOverrides.MutedText), ("open", Ui.ThemeOverrides.Open),
            ("new", Ui.ThemeOverrides.New), ("applied", Ui.ThemeOverrides.Applied), ("warning", Ui.ThemeOverrides.Warning), ("error", Ui.ThemeOverrides.Error)
        })
        {
            if (value is not null && !IsSupportedColour(value)) return Invalid($"ui.theme_overrides.{token}", "must be a named ANSI colour or #RRGGBB");
        }

        return new ConfigurationValidationResult.Valid();
    }

    private ConfigurationValidationResult ValidateKeybindings()
    {
        var bindings = new[] { ("scan", Keybindings.Scan), ("search", Keybindings.Search), ("filter", Keybindings.Filter),
            ("toggle_applied", Keybindings.ToggleApplied), ("history", Keybindings.History), ("open", Keybindings.Open),
            ("copy", Keybindings.Copy), ("help", Keybindings.Help), ("quit", Keybindings.Quit) };
        var values = new HashSet<string>(StringComparer.Ordinal);
        foreach (var (name, binding) in bindings)
        {
            if (binding.EnumerateRunes().Count() != 1) return Invalid($"keybindings.{name}", "must be exactly one character");
            var rune = binding.EnumerateRunes().Single();
            if (Rune.IsControl(rune)) return Invalid($"keybindings.{name}", "must be a non-control character");
            if (binding is "j" or "k" or "J" or "K") return Invalid($"keybindings.{name}", "must not collide with a fixed navigation key");
            if (!values.Add(binding)) return Invalid($"keybindings.{name}", "must not duplicate another keybinding");
        }

        return new ConfigurationValidationResult.Valid();
    }

    private static ConfigurationValidationResult ValidateSource(CompanyConfiguration company, int index)
    {
        var prefix = $"companies[{index}].source.";
        return company.Source switch
        {
            SourceConfiguration.Ashby source => NonEmpty(source.Board, prefix + "board"),
            SourceConfiguration.Greenhouse source => Then(NonEmpty(source.Board, prefix + "board"), () => OptionalCountry(source.CountryFilter, prefix + "country_filter")),
            SourceConfiguration.Jibe source => Then(Https(source.BaseUrl, prefix + "base_url"), () => NonEmpty(source.Client, prefix + "client")),
            SourceConfiguration.Ebay source => Https(source.ListingUrl, prefix + "listing_url"),
            SourceConfiguration.Recruitee source => Https(source.BaseUrl, prefix + "base_url"),
            SourceConfiguration.Personio source => Https(source.BaseUrl, prefix + "base_url"),
            SourceConfiguration.Lever source => Then(Https(source.ApiUrl, prefix + "api_url"), () => OptionalCountry(source.CountryFilter, prefix + "country_filter")),
            SourceConfiguration.Workable source => Then(NonEmpty(source.Account, prefix + "account"), () => OptionalCountry(source.CountryFilter, prefix + "country_filter")),
            SourceConfiguration.Workday source => Then(Https(source.BaseUrl, prefix + "base_url"), () => Then(NonEmpty(source.Tenant, prefix + "tenant"), () => Then(NonEmpty(source.Site, prefix + "site"), () => Then(NonEmpty(source.Country, prefix + "country"), () => ValidateCountry(source.CountryCode, prefix + "country_code"))))),
            SourceConfiguration.Yuki source => Exact(Equal(company.Id, "yuki") && Equal(source.FeedUrl, "https://jobs.yukisoftware.com/jobs.json"), Https(source.FeedUrl, prefix + "feed_url"), prefix + "feed_url", "must be Yuki's exact official JSON feed"),
            SourceConfiguration.Teamtailor source => Then(Https(source.FeedUrl, prefix + "feed_url"), () => NonEmpty(source.Employer, prefix + "employer")),
            SourceConfiguration.Bol source => Https(source.BaseUrl, prefix + "base_url"),
            SourceConfiguration.Coolblue source => Exact(Equal(source.ListingUrl, "https://www.coolblue.nl/vacatures/zoeken"), Https(source.ListingUrl, prefix + "listing_url"), prefix + "listing_url", "must be the official Coolblue Netherlands vacancy URL"),
            SourceConfiguration.Pay source => Exact(Equal(company.Id, "pay") && Equal(source.ListingUrl, "https://www.pay.nl/werk"), Https(source.ListingUrl, prefix + "listing_url"), prefix + "listing_url", "must be PAY.'s exact official vacancy URL"),
            SourceConfiguration.Buckaroo source => Exact(Equal(source.ListingUrl, "https://www.buckaroo.nl/over-buckaroo/vacatures"), Https(source.ListingUrl, prefix + "listing_url"), prefix + "listing_url", "must be the official Buckaroo vacancy URL"),
            SourceConfiguration.Rabobank source => Then(Https(source.BaseUrl, prefix + "base_url"), () => ValidateCountry(source.Country, prefix + "country")),
            SourceConfiguration.Eneco source => Https(source.ListingUrl, prefix + "listing_url"),
            SourceConfiguration.Exact source => ExactUrl(source.ListingUrl, "https://www.exact.com/careers/vacancies", prefix + "listing_url", "must be the official Exact vacancy URL"),
            SourceConfiguration.Afas source => ExactUrl(source.ListingUrl, "https://www.werkenbijafas.nl/alle-vacatures", prefix + "listing_url", "must be the official AFAS vacancy URL"),
            SourceConfiguration.Ns source => ExactUrl(source.ListingUrl, "https://www.werkenbijns.nl/vacatures", prefix + "listing_url", "must be the official NS vacancy URL"),
            SourceConfiguration.Achmea source => ExactUrl(source.ListingUrl, "https://www.werkenbijachmea.nl/vacatures", prefix + "listing_url", "must be the official Achmea vacancy URL"),
            SourceConfiguration.Chipsoft source => ExactUrl(source.ListingUrl, "https://www.chipsoft.com/nl-NL/werken-bij/vacatures", prefix + "listing_url", "must be the official ChipSoft vacancy URL"),
            SourceConfiguration.Anwb source => ExactUrl(source.FeedUrl, "https://www.werkenbijanwb.nl/fuse/vacancies.json", prefix + "feed_url", "must be the official ANWB vacancy feed"),
            SourceConfiguration.Postnl source => ExactUrl(source.ApiUrl, "https://vacatures-website.postnl.nl/vacatures-widget/api/", prefix + "api_url", "must be the official PostNL vacancy API"),
            SourceConfiguration.Pggm source => ExactUrl(source.ListingUrl, "https://www.werkenbijpggm.nl/vacatures", prefix + "listing_url", "must be the official PGGM vacancy URL"),
            SourceConfiguration.Amazon source => ExactUrl(source.SearchUrl, "https://www.amazon.jobs/en/search.json?normalized_country_code%5B%5D=NLD&offset=0&result_limit=100", prefix + "search_url", "must be the exact official Amazon Netherlands search API"),
            SourceConfiguration.Uber source => ExactUrl(source.ApiUrl, "https://iaziqy.fa.ocs.oraclecloud.com/hcmRestApi/resources/latest/", prefix + "api_url", "must be the official Uber Oracle HCM API"),
            SourceConfiguration.Microsoft source => ExactUrl(source.SearchUrl, "https://apply.careers.microsoft.com/api/pcsx/search?domain=microsoft.com&query=&location=Netherlands&start=0&hl=en", prefix + "search_url", "must be the exact official Microsoft Netherlands search API"),
            SourceConfiguration.Deel source => ValidateDeel(source.BoardUrl, prefix + "board_url"),
            SourceConfiguration.Successfactors source => Exact(Equal(company.Id, "flatexdegiro") && Equal(source.ListingUrl, "https://jobs.flatexdegiro.com/search/?q=&locationsearch=NL") && Equal(source.Employer, "flatexDEGIRO AG"), Then(Https(source.ListingUrl, prefix + "listing_url"), () => NonEmpty(source.Employer, prefix + "employer")), prefix + "listing_url", "must match flatexDEGIRO's exact official NL SuccessFactors board"),
            SourceConfiguration.Google source => ExactUrl(source.SearchUrl, "https://www.google.com/about/careers/applications/jobs/results/?company=Google&location=Netherlands&sort_by=date", prefix + "search_url", "must be the exact official Google Netherlands search URL"),
            SourceConfiguration.SuccessfactorsApi source => Exact(Equal(company.Id, "worldline") && Equal(source.BaseUrl, "https://jobs.worldline.com"), Https(source.BaseUrl, prefix + "base_url"), prefix + "base_url", "must match Worldline's official SuccessFactors host"),
            SourceConfiguration.AlbertHeijn source => Https(source.BaseUrl, prefix + "base_url"),
            SourceConfiguration.Ing source => Https(source.ListingUrl, prefix + "listing_url"),
            SourceConfiguration.Getnoticed source => ValidateGetnoticed(company.Id, source, prefix),
            SourceConfiguration.PagedHtml source => Then(Https(source.ListingUrl, prefix + "listing_url"), () => Then(NonEmpty(source.OffsetParameter, prefix + "offset_parameter"), () => source.PageSize <= 0 ? Invalid(prefix + "page_size", "must be greater than zero") : new ConfigurationValidationResult.Valid())),
            SourceConfiguration.Unsupported source => Then(NonEmpty(source.Reason, prefix + "reason"), () => company.Enabled ? Invalid(prefix + "strategy", "unsupported sources must be disabled") : new ConfigurationValidationResult.Valid()),
            _ => throw new InvalidOperationException("Unknown source configuration variant.")
        };
    }

    private static ConfigurationValidationResult ValidateGetnoticed(string id, SourceConfiguration.Getnoticed source, string prefix)
    {
        var httpsResult = Https(source.BaseUrl, prefix + "base_url");
        if (httpsResult is not ConfigurationValidationResult.Valid) return httpsResult;
        var valid = (id, source.BaseUrl, source.CountryFilter) switch
        {
            ("abn-amro", "https://www.werkenbijabnamro.nl", "Nederland") => true,
            ("topicus", "https://www.werkenbijtopicus.nl", null) => true,
            ("brand-new-day", "https://werkenbij.brandnewday.nl", null) => true,
            _ => false
        };
        if (valid) return new ConfigurationValidationResult.Valid();
        return id is "abn-amro" or "topicus" or "brand-new-day"
            ? Invalid(prefix + "base_url", "must match the company's official Getnoticed careers URL and filter")
            : Invalid(prefix + "strategy", "getnoticed is supported only for approved official company boards");
    }

    private static ConfigurationValidationResult ValidateDeel(string value, string field)
    {
        var httpsResult = Https(value, field);
        if (httpsResult is not ConfigurationValidationResult.Valid) return httpsResult;
        var uri = new Uri(value, UriKind.Absolute);
        var segments = uri.AbsolutePath.Split('/', StringSplitOptions.RemoveEmptyEntries);
        return string.Equals(uri.Host, "jobs.deel.com", StringComparison.Ordinal) && segments.Length == 1 && string.IsNullOrEmpty(uri.Query) && string.IsNullOrEmpty(uri.Fragment)
            ? new ConfigurationValidationResult.Valid()
            : Invalid(field, "must be an official Deel company board URL");
    }

    private static ConfigurationValidationResult ExactUrl(string value, string expected, string field, string message) => Exact(Equal(value, expected), Https(value, field), field, message);
    private static ConfigurationValidationResult Exact(bool condition, ConfigurationValidationResult initial, string field, string message) => initial is not ConfigurationValidationResult.Valid ? initial : condition ? new ConfigurationValidationResult.Valid() : Invalid(field, message);
    private static ConfigurationValidationResult Then(ConfigurationValidationResult initial, Func<ConfigurationValidationResult> next) => initial is ConfigurationValidationResult.Valid ? next() : initial;
    private static ConfigurationValidationResult OptionalCountry(string? value, string field) => value is null ? new ConfigurationValidationResult.Valid() : ValidateCountry(value, field);
    private static ConfigurationValidationResult NonEmpty(string value, string field) => string.IsNullOrWhiteSpace(value) ? Invalid(field, "must not be empty") : new ConfigurationValidationResult.Valid();
    private static ConfigurationValidationResult Https(string value, string field) => NonEmpty(value, field) is not ConfigurationValidationResult.Valid ? Invalid(field, "must not be empty") : Uri.TryCreate(value, UriKind.Absolute, out var uri) && string.Equals(uri.Scheme, Uri.UriSchemeHttps, StringComparison.Ordinal) && !string.IsNullOrEmpty(uri.Host) ? new ConfigurationValidationResult.Valid() : Invalid(field, "must be an HTTPS URL");
    private static ConfigurationValidationResult ValidateCountry(string value, string field) => value.Length == 2 && value.All(static character => character is >= 'A' and <= 'Z') ? new ConfigurationValidationResult.Valid() : Invalid(field, "must be a two-letter uppercase ASCII country code");
    private static bool IsSupportedColour(string value) => IsHexColour(value) || SupportedColours.Contains(value.Replace("-", string.Empty, StringComparison.Ordinal).Replace("_", string.Empty, StringComparison.Ordinal).ToLowerInvariant().Replace("grey", "gray", StringComparison.Ordinal).Replace("bright", "light", StringComparison.Ordinal));
    private static bool IsHexColour(string value) => value.Length == 7 && value[0] == '#' && value.Skip(1).All(static character => char.IsAsciiHexDigit(character));
    private static bool Equal(string left, string right) => string.Equals(left, right, StringComparison.Ordinal);
    private static ConfigurationValidationResult.Invalid Invalid(string field, string message) => new(field, message);
}
