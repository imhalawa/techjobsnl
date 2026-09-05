using System.Collections.Immutable;
using FluentAssertions;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Tests.Domain;

public sealed class VacancyAndScanModelTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-003")]
    [Trait("Category", "Unit")]
    public void VacancyRecord_PreservesTheRustIdentityObservationClassificationAndLifecycleFields()
    {
        var publishedAt = new DateTimeOffset(2026, 9, 5, 8, 30, 0, TimeSpan.Zero);
        var observation = new ObservedVacancy(
            "ats-42",
            "Platform Engineer",
            "Engineering",
            "Developer Experience",
            "Full-time",
            ["Amsterdam", "Remote"],
            ["NL"],
            "https://careers.example.test/jobs/ats-42",
            "https://careers.example.test/jobs/ats-42/apply",
            "Build reliable platforms.",
            "{\"id\":\"ats-42\"}",
            publishedAt);
        var classified = new ClassifiedVacancy(observation, new Eligibility(true, "eligible"));
        var record = new VacancyRecord(
            new VacancyKey("example", observation.SourceId),
            classified,
            true,
            true,
            publishedAt.AddDays(-2),
            publishedAt,
            null,
            null,
            publishedAt.AddHours(1));

        record.Key.Should().Be(new VacancyKey("example", "ats-42"));
        record.Classified.Observed.Locations.Should().Equal("Amsterdam", "Remote");
        record.Classified.Observed.Countries.Should().Equal("NL");
        record.Classified.Observed.RawPayload.Should().Be("{\"id\":\"ats-42\"}");
        record.Classified.Eligibility.Should().Be(new Eligibility(true, "eligible"));
        record.SourceOpen.Should().BeTrue();
        record.IsNew.Should().BeTrue();
        record.FirstSeenAt.Should().Be(publishedAt.AddDays(-2));
        record.LastSeenAt.Should().Be(publishedAt);
        record.ClosedAt.Should().BeNull();
        record.ReopenedAt.Should().BeNull();
        record.AppliedAt.Should().Be(publishedAt.AddHours(1));
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-003")]
    [Trait("Category", "Unit")]
    public void SourceScan_ExhaustivelyRepresentsCompleteAndIncompleteObservations()
    {
        var observation = Observation();
        SourceScan complete = new SourceScan.Complete([observation]);
        SourceScan incomplete = new SourceScan.Incomplete([observation], "The source response was truncated.");

        complete.Should().BeOfType<SourceScan.Complete>()
            .Which.Observations.Should().ContainSingle().Which.Should().Be(observation);
        incomplete.Should().BeOfType<SourceScan.Incomplete>()
            .Which.Diagnostic.Should().Be("The source response was truncated.");
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-003")]
    [Trait("Category", "Unit")]
    public void SourceErrorKind_ContainsEveryRustFailureKind()
    {
        Enum.GetValues<SourceErrorKind>().Should().Equal(
            SourceErrorKind.Configuration,
            SourceErrorKind.Transport,
            SourceErrorKind.Timeout,
            SourceErrorKind.RateLimit,
            SourceErrorKind.Schema,
            SourceErrorKind.IncompleteResults,
            SourceErrorKind.Browser,
            SourceErrorKind.Storage);
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-003")]
    [Trait("Category", "Unit")]
    public void ScanEvent_ExhaustivelyPreservesRunCompanyAndLegacySourceProgress()
    {
        var sourceScan = new SourceScan.Complete([Observation()]);
        ScanEvent[] events =
        [
            new RunStarted("run-7", 2),
            new CompanyStarted("example"),
            new CompanyCompleted("example", 3, 2),
            new CompanyFailed("broken", SourceErrorKind.Transport, "Connection reset."),
            new CompanyIncomplete("partial", "Missing page two.", 1),
            new RunFinished("run-7", 1, 1, 1),
            new Started("example"),
            new Completed("example", sourceScan),
            new Failed("broken", SourceErrorKind.Storage, "Transaction failed.")
        ];

        events.Select(@event => @event.GetType()).Should().Equal(
            typeof(RunStarted),
            typeof(CompanyStarted),
            typeof(CompanyCompleted),
            typeof(CompanyFailed),
            typeof(CompanyIncomplete),
            typeof(RunFinished),
            typeof(Started),
            typeof(Completed),
            typeof(Failed));
        ((CompanyFailed)events[3]).Kind.Should().Be(SourceErrorKind.Transport);
        ((Completed)events[7]).SourceScan.Should().Be(sourceScan);
    }

    private static ObservedVacancy Observation() => new(
        "ats-42",
        "Platform Engineer",
        null,
        null,
        null,
        ImmutableArray<string>.Empty,
        ImmutableArray<string>.Empty,
        "https://careers.example.test/jobs/ats-42",
        "https://careers.example.test/jobs/ats-42/apply",
        "Build reliable platforms.",
        "{\"id\":\"ats-42\"}",
        null);
}
