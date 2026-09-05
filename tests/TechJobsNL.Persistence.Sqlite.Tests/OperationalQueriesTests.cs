using System.Collections.Immutable;
using System.Globalization;
using FluentAssertions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Operations;
using TechJobsNL.Core.Ports.Persistence;

namespace TechJobsNL.Persistence.Sqlite.Tests;

[Trait("TaskId", "V0.1.0-018")]
[Trait("Category", "Integration")]
public sealed class OperationalQueriesTests
{
    [Fact]
    public async Task SourceHealth_NamesDifferOnlyByCase_OrdersByCompanyId()
    {
        var path = TemporaryPath();
        try
        {
            await using var store = await OpenAsync(path);
            ImmutableArray<CompanyConfiguration> configuration =
                [Company("zulu", "alpha"), Company("beta", "Beta"), Company("able", "ALPHA")];
            await store.SynchronizeCompaniesAsync(configuration, Token);
            IOperationalQueries queries = store;

            var sources = await queries.GetSourcesAsync(configuration, Token);

            sources.Select(static source => source.CompanyId.Value).Should().Equal("able", "zulu", "beta");
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task OperationalQueries_CancelledRequest_PropagatesCancellation()
    {
        var path = TemporaryPath();
        try
        {
            await using var store = await OpenAsync(path);
            IOperationalQueries queries = store;
            using var cancellation = new CancellationTokenSource();
            await cancellation.CancelAsync();

            var scans = () => queries.GetRecentScansAsync(cancellation.Token);
            var sources = () => queries.GetSourcesAsync([], cancellation.Token);

            await scans.Should().ThrowAsync<OperationCanceledException>();
            await sources.Should().ThrowAsync<OperationCanceledException>();
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task OperationalQueries_EmptyDatabase_ReturnsEmptyResults()
    {
        var path = TemporaryPath();
        try
        {
            await using var store = await OpenAsync(path);
            IOperationalQueries queries = store;

            (await queries.GetRecentScansAsync(Token)).Should().BeEmpty();
            (await queries.GetSourcesAsync([], Token)).Should().BeEmpty();
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task RecentScans_MoreThanOneHundredWithTies_ReturnsNewestCompletionsThenInsertions()
    {
        var path = TemporaryPath();
        try
        {
            await using var store = await OpenAsync(path);
            await store.SynchronizeCompaniesAsync([Company("alpha", "Alpha")], Token);
            await store.RecordCompleteScanAsync("newest", new CompanyId("alpha"), 0, At(11), At(12), Token);
            for (var index = 0; index < 102; index++)
            {
                await store.RecordCompleteScanAsync("tie-" + index.ToString("D3", CultureInfo.InvariantCulture),
                    new CompanyId("alpha"), 0, At(8), At(9), Token);
            }
            await store.RecordCompleteScanAsync("oldest-inserted-last", new CompanyId("alpha"), 0, At(7), At(8), Token);
            IOperationalQueries queries = store;

            var scans = await queries.GetRecentScansAsync(Token);

            scans.Should().HaveCount(100);
            scans[0].RunId.Should().Be("newest");
            scans.Skip(1).Select(static scan => scan.RunId).Should().Equal(
                Enumerable.Range(3, 99).Reverse().Select(static index => "tie-" + index.ToString("D3", CultureInfo.InvariantCulture)));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Theory]
    [InlineData("request failed token=private-value", "request failed token=[redacted]")]
    [InlineData("request failed Authorization: Bearer private-value", "request failed Authorization=[redacted]")]
    [InlineData("request failed https://user:password@example.test/jobs?key=private-value#private-fragment", "request failed https://example.test/jobs?[redacted]")]
    [InlineData("invalid response: {\"name\":\"private payload\"}", "invalid response: [redacted payload]")]
    [InlineData("request failed access_token=private-value", "request failed access_token=[redacted]")]
    [InlineData("request failed api-key=\"private value\"", "request failed api-key=[redacted]")]
    [InlineData("request failed Bearer private-value", "request failed Bearer [redacted]")]
    [InlineData("invalid response: <html>private payload</html>", "invalid response: [redacted payload]")]
    [InlineData("invalid response: [\"private payload\"]", "invalid response: [redacted payload]")]
    [InlineData("request failed Cookie: session=private-value", "request failed Cookie=[redacted]")]
    public async Task OperationalQueries_SensitiveDiagnostic_RedactsHistoryAndHealthAfterRestart(
        string diagnostic, string expected)
    {
        var path = TemporaryPath();
        var company = Company("alpha", "Alpha");
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([company], Token);
                await store.RecordFailedScanAsync("failed", new CompanyId("alpha"),
                    new ScanFailure(SourceErrorKind.Transport, diagnostic), At(8), At(9), Token);
            }

            await using var reopened = await OpenAsync(path);
            IOperationalQueries queries = reopened;
            (await queries.GetRecentScansAsync(Token)).Single().Diagnostic.Should().Be(expected);
            (await queries.GetSourcesAsync([company], Token)).Single().Diagnostic.Should().Be(expected);
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task SourceHealth_AfterRestart_PreservesEveryStateAndRetiredCompanies()
    {
        var path = TemporaryPath();
        ImmutableArray<CompanyConfiguration> configuration =
            [Company("gamma", "Gamma"), Company("beta", "Beta", false), Company("alpha", "Alpha")];
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync(configuration.Add(Company("delta", "Delta")), Token);
                await store.RecordCompleteScanAsync("alpha-ok", new CompanyId("alpha"), 1, At(8), At(9), Token);
                await store.RecordCompleteScanAsync("gamma-ok", new CompanyId("gamma"), 1, At(8), At(9), Token);
                await store.RecordCompleteScanAsync("delta-ok", new CompanyId("delta"), 1, At(8), At(9), Token);
                await store.RecordIncompleteScanAsync("gamma-partial", new CompanyId("gamma"), "unresolved location", 7, At(9), At(10), Token);
                await store.RecordFailedScanAsync("delta-failed", new CompanyId("delta"), new ScanFailure(SourceErrorKind.Transport, "connection reset"), At(10), At(11), Token);
                await store.SynchronizeCompaniesAsync(configuration, Token);
            }

            await using var reopened = await OpenAsync(path);
            IOperationalQueries queries = reopened;
            var sources = await queries.GetSourcesAsync(configuration, Token);

            sources.Should().Equal(
                new SourceOperationalView(new CompanyId("alpha"), "Alpha", "Ashby", true, At(9), At(9), OperationalHealth.Healthy, null, null),
                new SourceOperationalView(new CompanyId("beta"), "Beta", "Ashby", false, null, null, OperationalHealth.Unknown, null, null),
                new SourceOperationalView(new CompanyId("delta"), "Delta", "Unknown", false, At(11), At(9), OperationalHealth.Failed, SourceErrorKind.Transport, "connection reset"),
                new SourceOperationalView(new CompanyId("gamma"), "Gamma", "Ashby", true, At(10), At(9), OperationalHealth.Incomplete, SourceErrorKind.IncompleteResults, "unresolved location"));
        }
        finally
        {
            File.Delete(path);
        }
    }

    [Fact]
    public async Task RecentScans_AfterRestart_ReturnsCanonicalOutcomesAndDiagnostics()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("mollie", "Mollie")], Token);
                await store.RecordCompleteScanAsync("complete", new CompanyId("mollie"), 12, At(8), At(9), Token);
                await store.RecordIncompleteScanAsync("partial", new CompanyId("mollie"), "unresolved location", 7, At(9), At(10), Token);
                await store.RecordFailedScanAsync("failed", new CompanyId("mollie"), new ScanFailure(SourceErrorKind.Timeout, "timed out"), At(10), At(11), Token);
            }

            await using var reopened = await OpenAsync(path);
            IOperationalQueries queries = reopened;
            var scans = await queries.GetRecentScansAsync(Token);

            scans.Should().Equal(
                new ScanOperationalView("failed", new CompanyId("mollie"), "Mollie", At(10), At(11), OperationalOutcome.Failed, 0, SourceErrorKind.Timeout, "timed out"),
                new ScanOperationalView("partial", new CompanyId("mollie"), "Mollie", At(9), At(10), OperationalOutcome.Incomplete, 7, SourceErrorKind.IncompleteResults, "unresolved location"),
                new ScanOperationalView("complete", new CompanyId("mollie"), "Mollie", At(8), At(9), OperationalOutcome.Complete, 12, null, null));
        }
        finally
        {
            File.Delete(path);
        }
    }

    private static CancellationToken Token => TestContext.Current.CancellationToken;

    private static CompanyConfiguration Company(string id, string name, bool enabled = true) =>
        new(id, name, "Unknown", "Unknown", enabled, ImmutableDictionary<string, string>.Empty, new SourceConfiguration.Ashby(id));

    private static DateTimeOffset At(int hour) => new(2026, 8, 11, hour, 0, 0, TimeSpan.Zero);

    private static string TemporaryPath() => Path.Combine(Path.GetTempPath(), $"techjobsnl-operations-{Guid.NewGuid():N}.sqlite3");

    private static async Task<SqliteStore> OpenAsync(string path) =>
        await SqliteDatabase.OpenAsync(path, Token).ConfigureAwait(false) is SqliteOpenResult.Opened opened
            ? opened.Store
            : throw new InvalidOperationException("Database did not open.");
}
