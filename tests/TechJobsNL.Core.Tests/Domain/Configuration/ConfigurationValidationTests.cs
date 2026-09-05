using System.Collections.Immutable;
using FluentAssertions;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Core.Tests.Domain.Configuration;

public sealed class ConfigurationValidationTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    public void Validate_ArchivedConfiguration_PassesWithAllSixtySixCompanies()
    {
        var path = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "../../../../../archive/rust/config.toml"));
        var configuration = ArchivedConfigurationMapper.Load(File.ReadAllText(path));

        configuration.Companies.Should().HaveCount(66);
        configuration.Companies.Should().ContainSingle(company => !company.Enabled && company.Id == "coolblue");
        configuration.Validate().Should().BeOfType<ConfigurationValidationResult.Valid>();
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    [InlineData(0, "schema_version")]
    [InlineData(2, "schema_version")]
    public void Validate_InvalidSchemaVersion_ReturnsExactDiagnostic(int schemaVersion, string field)
    {
        var result = Valid(schemaVersion: schemaVersion).Validate();

        result.Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid(field, "must be exactly 1"));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    public void Validate_FilterAndAnalyticsFailures_PreserveRustOrderAndPaths()
    {
        Valid(filters: new FiltersConfiguration(ImmutableArray<string>.Empty, 0, [], [])).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("filters.new_job_max_age_days", "must be greater than zero"));
        Valid(filters: new FiltersConfiguration(["nl"], 7, ["("], [])).Validate()
            .Should().BeOfType<ConfigurationValidationResult.Invalid>().Which.Field.Should().Be("filters.countries[0]");
        Valid(filters: new FiltersConfiguration(["NL"], 7, ["("], [])).Validate()
            .Should().BeOfType<ConfigurationValidationResult.Invalid>().Which.Field.Should().Be("filters.include_title_patterns[0]");
        Valid(analytics: new AnalyticsConfiguration(AnalyticsProvider.Local, 0, 0, 0, 0)).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("analytics.minimum_cooccurrence", "must be greater than zero"));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    public void Validate_KeybindingsAndThemeCases_ReturnExactDiagnosticPaths()
    {
        Valid(ui: new UiConfiguration("other", true, ThemeOverrides.Empty)).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("ui.theme", "must be one of: clean-dark, clean-light"));
        Valid(ui: new UiConfiguration("clean-dark", true, new ThemeOverrides(null, "10", null, null, null, null, null, null, null, null, null))).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("ui.theme_overrides.focused_border", "must be a named ANSI colour or #RRGGBB"));
        Valid(keybindings: new KeybindingsConfiguration("j", "/", "f", "a", "h", "o", "c", "?", "q")).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("keybindings.scan", "must not collide with a fixed navigation key"));
        Valid(keybindings: new KeybindingsConfiguration("r", "/", "f", "a", "h", "o", "o", "?", "q")).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("keybindings.copy", "must not duplicate another keybinding"));
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    [InlineData("darkgray")]
    [InlineData("LightRed")]
    [InlineData("bright-red")]
    [InlineData("dark-grey")]
    [InlineData("light_green")]
    [InlineData("#ff5555")]
    public void Validate_RatatuiCompatibleColours_AcceptsCharacterizedVariants(string colour)
    {
        var result = Valid(ui: new UiConfiguration("clean-dark", true, new ThemeOverrides(null, colour, null, null, null, null, null, null, null, null, null))).Validate();

        result.Should().BeOfType<ConfigurationValidationResult.Valid>();
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    public void Validate_SourceCompatibilityAndTrustedEndpoints_EnforcesExactRustRules()
    {
        Valid(source: new SourceConfiguration.Unsupported("blocked"), enabled: true).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("companies[0].source.strategy", "unsupported sources must be disabled"));
        Valid(source: new SourceConfiguration.PagedHtml("https://example.test", "offset", 0)).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("companies[0].source.page_size", "must be greater than zero"));
        Valid(source: new SourceConfiguration.Getnoticed("https://www.werkenbijabnamro.nl", "Nederland"), companyId: "other").Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("companies[0].source.strategy", "getnoticed is supported only for approved official company boards"));
        Valid(source: new SourceConfiguration.Microsoft("http://apply.careers.microsoft.com/api/pcsx/search")).Validate()
            .Should().BeEquivalentTo(new ConfigurationValidationResult.Invalid("companies[0].source.search_url", "must be an HTTPS URL"));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-005")]
    [Trait("Category", "Unit")]
    public void Defaults_DeferredValues_AreRetainedWithoutExecution()
    {
        AnalyticsConfiguration.Default.Should().BeEquivalentTo(new AnalyticsConfiguration(AnalyticsProvider.Local, 2, 50, 60, 3));
        FiltersConfiguration.DefaultNewJobMaxAgeDays.Should().Be(7);
        KeybindingsConfiguration.DefaultCopy.Should().Be("c");
        CompanyConfiguration.UnknownMetadata.Should().Be("Unknown");
    }

    private static AppConfiguration Valid(int schemaVersion = 1, string companyId = "mollie", bool enabled = true,
        SourceConfiguration? source = null, FiltersConfiguration? filters = null, AnalyticsConfiguration? analytics = null,
        UiConfiguration? ui = null, KeybindingsConfiguration? keybindings = null) => new(
        schemaVersion, ":memory:", [new CompanyConfiguration(companyId, "Mollie", "Test", "Test", enabled,
            ImmutableDictionary<string, string>.Empty, source ?? new SourceConfiguration.Ashby("mollie"))],
        filters ?? new FiltersConfiguration(["NL"], 7, [], []), new ScanConfiguration(1, 20, 0, "test"),
        analytics ?? AnalyticsConfiguration.Default, ui ?? new UiConfiguration("clean-dark", true, ThemeOverrides.Empty),
        keybindings ?? new KeybindingsConfiguration("r", "/", "f", "a", "h", "o", "c", "?", "q"));
}
