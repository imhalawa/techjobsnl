using FluentAssertions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Profiles;

namespace TechJobsNL.Core.Tests.Profiles;

public sealed class CompanyProjectionsTests
{
    private static readonly DateTimeOffset At10 = new(2026, 8, 11, 10, 0, 0, TimeSpan.Zero);

    [Fact]
    [Trait("TaskId", "V0.1.0-015")]
    public void UpdateFeed_ProjectsAllTypesWithStableChronologyAndMeaningfulEvidenceOnly()
    {
        var closed = Vacancy("closed", At10, open: false, closedAt: At10.AddHours(2));
        var reopened = Vacancy("reopened", At10.AddMinutes(1), reopenedAt: At10.AddHours(3));
        var snapshots = new[] { Snapshot(closed.Key, "hash-1", At10, "Engineer"), Snapshot(closed.Key, "hash-2", At10.AddHours(1), "Principal Engineer"), Snapshot(reopened.Key, "hash-a", At10.AddMinutes(1), "Designer") };

        var feed = CompanyProjections.UpdateFeed([Facts(true)], [closed, reopened], snapshots);

        feed.Select(update => update.Kind).Should().Equal(VacancyUpdateKind.Reopened, VacancyUpdateKind.Closed, VacancyUpdateKind.Changed, VacancyUpdateKind.New, VacancyUpdateKind.New);
        feed.Count(update => update.Kind == VacancyUpdateKind.Changed).Should().Be(1);
        feed.Single(update => update.Kind == VacancyUpdateKind.Changed).ContentHash.Should().Be("hash-2");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-015")]
    public void Profiles_UnfollowedCompanyRetainsCurrentJobsHealthLinksAndFeedHistory()
    {
        var vacancy = Vacancy("one", At10);
        var facts = Facts(false);

        var profile = CompanyProjections.Profiles([facts], [vacancy]).Single();
        var feed = CompanyProjections.UpdateFeed([facts], [vacancy], [Snapshot(vacancy.Key, "hash", At10, "Engineer")]);

        profile.Should().Match<CompanyProfile>(value => !value.IsFollowed && value.Health == CompanySourceHealth.Incomplete && value.Diagnostic == "truncated" && value.OfficialSource == new Uri("https://jobs.example.test"));
        profile.CurrentVacancies.Should().ContainSingle();
        feed.Should().ContainSingle().Which.Kind.Should().Be(VacancyUpdateKind.New);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-015")]
    public void UpdateFeed_EqualTimes_UseStableCompanyAndSourceTies()
    {
        var first = Vacancy("b", At10);
        var second = Vacancy("a", At10);

        CompanyProjections.UpdateFeed([Facts(true)], [first, second], []).Select(update => update.Key.SourceId.Value).Should().Equal("a", "b");
    }

    private static CompanyProfileFacts Facts(bool followed) => new(new CompanyId("alpha"), "Alpha", followed, new Uri("https://jobs.example.test"), CompanySourceHealth.Incomplete, "truncated");
    private static VacancySnapshotEvidence Snapshot(VacancyKey key, string hash, DateTimeOffset at, string title) => new(key, hash, at, title);
    private static VacancyRecord Vacancy(string id, DateTimeOffset firstSeen, bool open = true, DateTimeOffset? closedAt = null, DateTimeOffset? reopenedAt = null) =>
        new(new VacancyKey(new CompanyId("alpha"), new SourceId(id)), new ClassifiedVacancy(new ObservedVacancy(new SourceId(id), "Engineer", null, null, null, ["Amsterdam"], ["NL"], $"https://example.test/{id}", $"https://example.test/{id}/apply", "Description", "{}", null), new TechJobsNL.Core.Domain.Eligibility(true, "eligible")), open, true, firstSeen, firstSeen, closedAt, reopenedAt, null);
}
