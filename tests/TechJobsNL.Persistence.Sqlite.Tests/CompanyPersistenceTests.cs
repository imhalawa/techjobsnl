using System.Collections.Immutable;
using FluentAssertions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Persistence.Sqlite.Tests;

public sealed class CompanyPersistenceTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-011")]
    public async Task SynchronizeCompaniesAsync_RemovedCompanyIsDisabledWithoutLosingHealth()
    {
        var path = Path.Combine(Path.GetTempPath(), $"techjobsnl-{Guid.NewGuid():N}.sqlite3");
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("removed", true), Company("current", true)], TestContext.Current.CancellationToken);
                await store.RecordCompleteScanAsync("run-1", new CompanyId("removed"), 2, At(9), At(10), TestContext.Current.CancellationToken);
                await store.SynchronizeCompaniesAsync([Company("current", false)], TestContext.Current.CancellationToken);
            }

            await using var reopened = await OpenAsync(path);
            var health = await reopened.GetSourceHealthAsync(TestContext.Current.CancellationToken);
            var removed = health.Single(item => item.CompanyId == new CompanyId("removed"));
            removed.IsEnabled.Should().BeFalse();
            removed.Health.Should().Be(SourceHealth.Healthy);
            removed.LatestAttemptedAt.Should().Be(At(10));
            removed.LatestSuccessfulAt.Should().Be(At(10));
            health.Single(item => item.CompanyId == new CompanyId("current")).IsEnabled.Should().BeFalse();
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-011")]
    public async Task FailedAndIncompleteScans_PersistCompanyLocalDiagnosticsAndTimestamps()
    {
        var path = Path.Combine(Path.GetTempPath(), $"techjobsnl-{Guid.NewGuid():N}.sqlite3");
        try
        {
            await using var store = await OpenAsync(path);
            await store.SynchronizeCompaniesAsync([Company("alpha", true), Company("beta", true)], TestContext.Current.CancellationToken);
            await store.RecordCompleteScanAsync("run-1", new CompanyId("alpha"), 3, At(8), At(9), TestContext.Current.CancellationToken);
            await store.RecordIncompleteScanAsync("run-2", new CompanyId("alpha"), "page limit", 1, At(10), At(11), TestContext.Current.CancellationToken);
            await store.RecordFailedScanAsync("run-3", new CompanyId("beta"), new ScanFailure(SourceErrorKind.Transport, "connection reset"), At(12), At(13), TestContext.Current.CancellationToken);

            var sources = await store.GetSourceHealthAsync(TestContext.Current.CancellationToken);
            var alpha = sources.Single(item => item.CompanyId == new CompanyId("alpha"));
            alpha.Should().Match<SourceHealthRecord>(item => item.Health == SourceHealth.Incomplete && item.LatestAttemptedAt == At(11) && item.LatestSuccessfulAt == At(9) && item.Diagnostic == "page limit");
            sources.Single(item => item.CompanyId == new CompanyId("beta")).Should().Match<SourceHealthRecord>(item => item.Health == SourceHealth.Failed && item.Diagnostic == "connection reset");

            var history = await store.GetScanHistoryAsync(TestContext.Current.CancellationToken);
            history.Select(item => item.RunId).Should().Equal("run-3", "run-2", "run-1");
            history.Single(item => string.Equals(item.RunId, "run-2", StringComparison.Ordinal)).ErrorKind.Should().Be(SourceErrorKind.IncompleteResults);
            history.Single(item => string.Equals(item.RunId, "run-3", StringComparison.Ordinal)).Diagnostic.Should().Be("connection reset");
        }
        finally { File.Delete(path); }
    }

    private static async Task<SqliteStore> OpenAsync(string path) => (await SqliteDatabase.OpenAsync(path, TestContext.Current.CancellationToken).ConfigureAwait(false)) switch
    {
        SqliteOpenResult.Opened opened => opened.Store,
        var result => throw new InvalidOperationException($"Could not open test database: {result}")
    };

    private static CompanyConfiguration Company(string id, bool enabled) => new(id, id, "Unknown", "Unknown", enabled,
        ImmutableDictionary<string, string>.Empty, new SourceConfiguration.Unsupported("fixture"));

    private static DateTimeOffset At(int hour) => new(2026, 8, 11, hour, 0, 0, TimeSpan.Zero);
}
