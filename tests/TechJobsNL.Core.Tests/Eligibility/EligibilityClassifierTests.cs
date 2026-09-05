using FluentAssertions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Eligibility;

namespace TechJobsNL.Core.Tests.Classification;

public sealed class EligibilityClassifierTests
{
    [Theory]
    [Trait("TaskId", "V0.1.0-007")]
    [InlineData("Senior Platform Engineer", "NL", true, "eligible")]
    [InlineData("Software Engineer", "PT", false, "outside-configured-countries")]
    [InlineData("Engineering Manager", "NL", false, "excluded-title")]
    [InlineData("Accountant", "NL", false, "not-included-title")]
    public void Classify_RustCases_PreserveDecisionOrder(string title, string country, bool eligible, string reason)
    {
        var classifier = Ready(Filters());

        classifier.Classify(Observed(title, ["Amsterdam"], [country]), new Dictionary<string, string>(StringComparer.Ordinal))
            .Should().Be(new EligibilityClassification.Decided(new TechJobsNL.Core.Domain.Eligibility(eligible, reason)));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-007")]
    public void Classify_LocationOverride_ResolvesCountry()
    {
        Ready(Filters()).Classify(Observed("Software Engineer", ["Amsterdam"], []), new Dictionary<string, string>(StringComparer.Ordinal) { ["Amsterdam"] = "NL" })
            .Should().Be(new EligibilityClassification.Decided(new TechJobsNL.Core.Domain.Eligibility(true, "eligible")));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-007")]
    public void Classify_UnresolvedLocation_IsTypedIncomplete()
    {
        Ready(Filters()).Classify(Observed("Software Engineer", ["Hybrid"], []), new Dictionary<string, string>(StringComparer.Ordinal))
            .Should().BeOfType<EligibilityClassification.Incomplete>()
            .Which.UnresolvedLocations.Should().Equal("Hybrid");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-007")]
    public void Create_InvalidRegex_IsRejectedWithPatternContext()
    {
        EligibilityClassifier.Create(Filters(include: ["("])).Should().BeOfType<EligibilityClassifierCreation.InvalidPattern>()
            .Which.Pattern.Should().Be("(");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-007")]
    public void Classify_EmptyPatterns_AdmitsNonEngineeringTitle()
    {
        Ready(Filters(include: [], exclude: [])).Classify(Observed("Senior Legal Counsel", ["Amsterdam"], ["NL"]), new Dictionary<string, string>(StringComparer.Ordinal))
            .Should().Be(new EligibilityClassification.Decided(new TechJobsNL.Core.Domain.Eligibility(true, "eligible")));
    }

    private static EligibilityClassifier Ready(FiltersConfiguration filters) =>
        ((EligibilityClassifierCreation.Ready)EligibilityClassifier.Create(filters)).Classifier;

    private static FiltersConfiguration Filters(System.Collections.Immutable.ImmutableArray<string>? include = null, System.Collections.Immutable.ImmutableArray<string>? exclude = null) =>
        new(["NL"], 7, include ?? ["software engineer|platform engineer", "principal engineer"], exclude ?? ["manager"]);

    private static ObservedVacancy Observed(string title, System.Collections.Immutable.ImmutableArray<string> locations, System.Collections.Immutable.ImmutableArray<string> countries) =>
        new(new SourceId("job-123"), title, "Engineering", "Platform", "Full-time", locations, countries,
            "https://example.test/jobs/123", "https://example.test/jobs/123/apply", "Description", "{}", DateTimeOffset.Parse("2026-08-11T09:30:00Z", System.Globalization.CultureInfo.InvariantCulture));
}
