using System.Collections.Immutable;
using FluentAssertions;
using Microsoft.Data.Sqlite;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Persistence.Sqlite.Tests;

public sealed class AtomicScanPersistenceTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-012")]
    public async Task PersistCompleteScanAsync_ClosesOnlyMissingVacanciesForItsCompany()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha"), Company("beta")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("a1", Company("alpha"), [Vacancy("one"), Vacancy("two")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("b1", Company("beta"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("a2", Company("alpha"), [Vacancy("one")], At(10), At(11), TestContext.Current.CancellationToken);
            }

            (await ScalarAsync<long>(path, "select source_open from jobs where company_id='alpha' and source_id='two';")).Should().Be(0);
            (await ScalarAsync<long>(path, "select source_open from jobs where company_id='beta' and source_id='one';")).Should().Be(1);
            (await ScalarAsync<long>(path, "select observed_count from scans where run_id='a2';")).Should().Be(1);
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-012")]
    public async Task IncompleteAndFailedScans_DoNotMutateTrustedVacancies()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("complete", Company("alpha"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.RecordIncompleteScanAsync("partial", new CompanyId("alpha"), "truncated", 0, At(10), At(11), TestContext.Current.CancellationToken);
                await store.RecordFailedScanAsync("failed", new CompanyId("alpha"), new ScanFailure(SourceErrorKind.Schema, "bad JSON"), At(12), At(13), TestContext.Current.CancellationToken);
            }

            (await ScalarAsync<long>(path, "select source_open from jobs where company_id='alpha' and source_id='one';")).Should().Be(1);
            (await ScalarAsync<string>(path, "select title from jobs where company_id='alpha' and source_id='one';")).Should().Be("Engineer one");
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-012")]
    public async Task PersistCompleteScanAsync_LateInjectedFailure_RollsBackVacancyChanges()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("baseline", Company("alpha"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
            }
            await ExecuteAsync(path, "create trigger fail_scan before insert on scans begin select raise(abort, 'forced scan failure'); end;");
            await using (var store = await OpenAsync(path))
            {
                var action = () => store.PersistCompleteScanAsync("broken", Company("alpha"), [Vacancy("one", "Changed")], At(10), At(11), TestContext.Current.CancellationToken);
                await action.Should().ThrowAsync<SqliteException>().WithMessage("*forced scan failure*");
            }

            (await ScalarAsync<string>(path, "select title from jobs where company_id='alpha' and source_id='one';")).Should().Be("Engineer one");
            (await ScalarAsync<long>(path, "select count(*) from scans;")).Should().Be(1);
        }
        finally { File.Delete(path); }
    }

    private static ClassifiedVacancy Vacancy(string id, string? title = null) => new(
        new ObservedVacancy(new SourceId(id), title ?? $"Engineer {id}", null, null, null, ["Amsterdam"], ["NL"], $"https://example.test/jobs/{id}", $"https://example.test/jobs/{id}/apply", "Description", "{}", null),
        new TechJobsNL.Core.Domain.Eligibility(true, "eligible"));
    private static CompanyConfiguration Company(string id) => new(id, id, "Unknown", "Unknown", true, ImmutableDictionary<string, string>.Empty, new SourceConfiguration.Unsupported("fixture"));
    private static DateTimeOffset At(int hour) => new(2026, 8, 11, hour, 0, 0, TimeSpan.Zero);
    private static string TemporaryPath() => Path.Combine(Path.GetTempPath(), $"techjobsnl-{Guid.NewGuid():N}.sqlite3");
    private static async Task<SqliteStore> OpenAsync(string path) => (await SqliteDatabase.OpenAsync(path, TestContext.Current.CancellationToken).ConfigureAwait(false)) is SqliteOpenResult.Opened opened ? opened.Store : throw new InvalidOperationException("Database did not open.");
    private static async Task ExecuteAsync(string path, string sql)
    {
        var connection = new SqliteConnection($"Data Source={path};Pooling=False");
        await using (connection.ConfigureAwait(false))
        {
            await connection.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            var command = connection.CreateCommand();
            await using (command.ConfigureAwait(false))
            {
                command.CommandText = sql;
                await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            }
        }
    }

    private static async Task<T> ScalarAsync<T>(string path, string sql)
    {
        var connection = new SqliteConnection($"Data Source={path};Pooling=False");
        await using (connection.ConfigureAwait(false))
        {
            await connection.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            var command = connection.CreateCommand();
            await using (command.ConfigureAwait(false))
            {
                command.CommandText = sql;
                return (T)(await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!;
            }
        }
    }
}
