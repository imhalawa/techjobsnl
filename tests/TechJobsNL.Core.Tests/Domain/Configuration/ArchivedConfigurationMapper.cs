using System.Collections.Immutable;
using TechJobsNL.Core.Domain.Configuration;
using Tomlyn;
using Tomlyn.Model;

namespace TechJobsNL.Core.Tests.Domain.Configuration;

internal static class ArchivedConfigurationMapper
{
    internal static AppConfiguration Load(string toml)
    {
        var root = TomlSerializer.Deserialize<TomlTable>(toml) ?? throw new InvalidOperationException("Archived TOML produced no root table.");
        var filters = Table(root, "filters");
        var scan = Table(root, "scan");
        var analytics = Table(root, "analytics");
        var ui = Table(root, "ui");
        var keybindings = Table(root, "keybindings");
        return new AppConfiguration(
            Number(root, "schema_version"), Text(root, "database_path"),
            Tables(root, "companies").Select(Company).ToImmutableArray(),
            new FiltersConfiguration(Strings(filters, "countries"), Number(filters, "new_job_max_age_days"), Strings(filters, "include_title_patterns"), Strings(filters, "exclude_title_patterns")),
            new ScanConfiguration(Number(scan, "concurrency"), Number(scan, "timeout_seconds"), Number(scan, "retry_count"), Text(scan, "user_agent")),
            new AnalyticsConfiguration(Provider(Text(analytics, "provider")), Number(analytics, "minimum_skill_occurrence"), Number(analytics, "maximum_skills"), Number(analytics, "ai_timeout_seconds"), Number(analytics, "minimum_cooccurrence")),
            new UiConfiguration(Text(ui, "theme"), Flag(ui, "unicode_icons"), Overrides(ui)),
            new KeybindingsConfiguration(Text(keybindings, "scan"), Text(keybindings, "search"), Text(keybindings, "filter"), Text(keybindings, "toggle_applied"), Text(keybindings, "history"), Text(keybindings, "open"), Text(keybindings, "copy"), Text(keybindings, "help"), Text(keybindings, "quit")));
    }

    private static CompanyConfiguration Company(TomlTable table) => new(Text(table, "id"), Text(table, "name"), TextOr(table, "industry", CompanyConfiguration.UnknownMetadata), TextOr(table, "scale", CompanyConfiguration.UnknownMetadata), Flag(table, "enabled"), TableOrEmpty(table, "location_country_overrides").ToImmutableDictionary(static pair => pair.Key, static pair => (string)pair.Value!, StringComparer.Ordinal), Source(Table(table, "source")));
    private static ThemeOverrides Overrides(TomlTable ui) { var table = TableOrEmpty(ui, "theme_overrides"); return new ThemeOverrides(TextOrNull(table, "background"), TextOrNull(table, "focused_border"), TextOrNull(table, "unfocused_border"), TextOrNull(table, "selected_row"), TextOrNull(table, "primary_text"), TextOrNull(table, "muted_text"), TextOrNull(table, "open"), TextOrNull(table, "new"), TextOrNull(table, "applied"), TextOrNull(table, "warning"), TextOrNull(table, "error")); }

    private static SourceConfiguration Source(TomlTable table)
    {
        var strategy = Text(table, "strategy");
        return strategy switch
        {
            "ashby" => new SourceConfiguration.Ashby(Text(table, "board")),
            "greenhouse" => new SourceConfiguration.Greenhouse(Text(table, "board"), TextOrNull(table, "country_filter")),
            "jibe" => new SourceConfiguration.Jibe(Text(table, "base_url"), Text(table, "client")),
            "ebay" => new SourceConfiguration.Ebay(Text(table, "listing_url")),
            "recruitee" => new SourceConfiguration.Recruitee(Text(table, "base_url")),
            "personio" => new SourceConfiguration.Personio(Text(table, "base_url")),
            "lever" => new SourceConfiguration.Lever(Text(table, "api_url"), TextOrNull(table, "country_filter")),
            "workable" => new SourceConfiguration.Workable(Text(table, "account"), TextOrNull(table, "country_filter")),
            "workday" => new SourceConfiguration.Workday(Text(table, "base_url"), Text(table, "tenant"), Text(table, "site"), Text(table, "country"), Text(table, "country_code")),
            "yuki" => new SourceConfiguration.Yuki(Text(table, "feed_url")),
            "teamtailor" => new SourceConfiguration.Teamtailor(Text(table, "feed_url"), Text(table, "employer")),
            "bol" => new SourceConfiguration.Bol(Text(table, "base_url")),
            "coolblue" => new SourceConfiguration.Coolblue(Text(table, "listing_url")),
            "pay" => new SourceConfiguration.Pay(Text(table, "listing_url")),
            "buckaroo" => new SourceConfiguration.Buckaroo(Text(table, "listing_url")),
            "rabobank" => new SourceConfiguration.Rabobank(Text(table, "base_url"), Text(table, "country")),
            "eneco" => new SourceConfiguration.Eneco(Text(table, "listing_url")),
            "exact" => new SourceConfiguration.Exact(Text(table, "listing_url")),
            "afas" => new SourceConfiguration.Afas(Text(table, "listing_url")),
            "ns" => new SourceConfiguration.Ns(Text(table, "listing_url")),
            "achmea" => new SourceConfiguration.Achmea(Text(table, "listing_url")),
            "chipsoft" => new SourceConfiguration.Chipsoft(Text(table, "listing_url")),
            "anwb" => new SourceConfiguration.Anwb(Text(table, "feed_url")),
            "postnl" => new SourceConfiguration.Postnl(Text(table, "api_url")),
            "pggm" => new SourceConfiguration.Pggm(Text(table, "listing_url")),
            "amazon" => new SourceConfiguration.Amazon(Text(table, "search_url")),
            "uber" => new SourceConfiguration.Uber(Text(table, "api_url")),
            "microsoft" => new SourceConfiguration.Microsoft(Text(table, "search_url")),
            "deel" => new SourceConfiguration.Deel(Text(table, "board_url")),
            "successfactors" => new SourceConfiguration.Successfactors(Text(table, "listing_url"), Text(table, "employer")),
            "google" => new SourceConfiguration.Google(Text(table, "search_url")),
            "successfactors-api" => new SourceConfiguration.SuccessfactorsApi(Text(table, "base_url")),
            "albert-heijn" => new SourceConfiguration.AlbertHeijn(Text(table, "base_url")),
            "ing" => new SourceConfiguration.Ing(Text(table, "listing_url")),
            "getnoticed" => new SourceConfiguration.Getnoticed(Text(table, "base_url"), TextOrNull(table, "country_filter")),
            "paged-html" => new SourceConfiguration.PagedHtml(Text(table, "listing_url"), Text(table, "offset_parameter"), Number(table, "page_size")),
            "unsupported" => new SourceConfiguration.Unsupported(Text(table, "reason")),
            _ => throw new InvalidOperationException($"Unknown archived strategy '{strategy}'.")
        };
    }

    private static TomlTable Table(TomlTable table, string key) => (TomlTable)table[key]!;
    private static TomlTable TableOrEmpty(TomlTable table, string key) => table.TryGetValue(key, out var value) ? (TomlTable)value! : new TomlTable();
    private static TomlArray Array(TomlTable table, string key) => (TomlArray)table[key]!;
    private static TomlTableArray Tables(TomlTable table, string key) => (TomlTableArray)table[key]!;
    private static string Text(TomlTable table, string key) => (string)table[key]!;
    private static string TextOr(TomlTable table, string key, string defaultValue) => table.TryGetValue(key, out var value) ? (string)value! : defaultValue;
    private static string? TextOrNull(TomlTable table, string key) => table.TryGetValue(key, out var value) ? (string)value! : null;
    private static int Number(TomlTable table, string key) => Convert.ToInt32(table[key], System.Globalization.CultureInfo.InvariantCulture);
    private static bool Flag(TomlTable table, string key) => (bool)table[key]!;
    private static ImmutableArray<string> Strings(TomlTable table, string key) => Array(table, key).Cast<string>().ToImmutableArray();
    private static AnalyticsProvider Provider(string value) => value switch { "local" => AnalyticsProvider.Local, "claude" => AnalyticsProvider.Claude, "codex" => AnalyticsProvider.Codex, _ => throw new InvalidOperationException($"Unknown analytics provider '{value}'.") };
}
