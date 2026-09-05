using FluentAssertions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.Core.Tests.Vacancies;

public sealed class VacancyViewsTests
{
    private static readonly DateTimeOffset Now = new(2026, 8, 20, 12, 0, 0, TimeSpan.Zero);
    private static readonly CompanyView[] Companies = [new(new CompanyId("alpha"), "Alpha Labs", true), new(new CompanyId("disabled"), "Hidden Corp", false)];

    [Theory]
    [Trait("TaskId", "V0.1.0-014")]
    [InlineData(VacancyView.Active, "active,applied,reopened")]
    [InlineData(VacancyView.New, "active")]
    [InlineData(VacancyView.Applied, "applied")]
    [InlineData(VacancyView.History, "closed,reopened")]
    [InlineData(VacancyView.All, "active,applied,closed,reopened,disabled,ineligible")]
    public void Query_ViewMembership_MatchesRust(VacancyView view, string expected)
    {
        var rows = new[] { Vacancy("active", published: Now.AddDays(-1)), Vacancy("applied", applied: Now), Vacancy("closed", open: false), Vacancy("reopened", reopened: Now.AddHours(-1)), Vacancy("disabled", company: "disabled"), Vacancy("ineligible", eligible: false) };

        VacancyViews.Query(rows, Companies, view, string.Empty, Now, 7).Select(row => row.Key.SourceId.Value).Should().BeEquivalentTo(expected.Split(','));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-014")]
    public void Query_NewView_ExcludesMissingFutureAndOldPublicationDates()
    {
        var rows = new[] { Vacancy("missing"), Vacancy("future", published: Now.AddMinutes(1)), Vacancy("old", published: Now.AddDays(-8)), Vacancy("current", published: Now.AddDays(-7)) };

        VacancyViews.Query(rows, Companies, VacancyView.New, string.Empty, Now, 7).Select(row => row.Key.SourceId.Value).Should().Equal("current");
    }

    [Theory]
    [Trait("TaskId", "V0.1.0-014")]
    [InlineData("PLATFORM", "one")]
    [InlineData("alpha LABS", "one,two")]
    [InlineData("needle", "")]
    public void Query_SearchUsesOnlyTitleCompanyIdAndDisplayName(string search, string expected)
    {
        var rows = new[] { Vacancy("one", title: "Platform Engineer", description: "ordinary"), Vacancy("two", title: "Designer", description: "needle") };

        VacancyViews.Query(rows, Companies, VacancyView.Active, search, Now, 7).Select(row => row.Key.SourceId.Value).Should().BeEquivalentTo(expected.Length == 0 ? [] : expected.Split(','));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-014")]
    public void ActiveCount_UsesCanonicalMembership()
    {
        VacancyViews.ActiveCount([Vacancy("one"), Vacancy("two", open: false), Vacancy("three", company: "disabled")], Companies).Should().Be(1);
    }

    private static VacancyRecord Vacancy(string id, string company = "alpha", string? title = null, string description = "description", bool open = true, bool eligible = true, DateTimeOffset? published = null, DateTimeOffset? reopened = null, DateTimeOffset? applied = null) =>
        new(new VacancyKey(new CompanyId(company), new SourceId(id)), new ClassifiedVacancy(new ObservedVacancy(new SourceId(id), title ?? id, null, null, null, ["Amsterdam"], ["NL"], $"https://example.test/{id}", $"https://example.test/{id}/apply", description, "{}", published), new TechJobsNL.Core.Domain.Eligibility(eligible, eligible ? "eligible" : "excluded-title")), open, true, Now.AddDays(-2), Now, open ? null : Now, reopened, applied);
}
