using System.Collections.Immutable;
using FluentAssertions;
using Microsoft.Data.Sqlite;
using System.Text.Json.Nodes;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Persistence.Sqlite.Tests;

public sealed class AtomicScanPersistenceTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-016")]
    public async Task AppliedAndSavedToggles_ReturnCanonicalStateAndSurviveRestart()
    {
        var path = TemporaryPath();
        try
        {
            var key = new VacancyKey(new CompanyId("alpha"), new SourceId("one"));
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("first", Company("alpha"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
                (await store.ToggleAppliedAsync(key, At(10), TestContext.Current.CancellationToken)).Should().Be(new AppliedToggleResult(key, true, At(10)));
                (await store.ToggleSavedVacancyAsync(key, TestContext.Current.CancellationToken)).Should().Be(new SavedVacancyToggleResult(key, true));
            }

            await using var reopened = await OpenAsync(path);
            (await reopened.GetAllVacanciesAsync(TestContext.Current.CancellationToken)).Single().AppliedAt.Should().Be(At(10));
            var library = JsonNode.Parse((await reopened.GetLibraryJsonAsync(TestContext.Current.CancellationToken))!)!.AsObject();
            library["jobs"]!.AsArray().Should().ContainSingle();
            (await reopened.ToggleAppliedAsync(key, At(11), TestContext.Current.CancellationToken)).IsApplied.Should().BeFalse();
            (await reopened.ToggleSavedVacancyAsync(key, TestContext.Current.CancellationToken)).IsSaved.Should().BeFalse();
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-016")]
    public async Task ToggleSavedVacancyAsync_PreservesDeferredLibraryAndFiltersWithoutOrphans()
    {
        var path = TemporaryPath();
        const string libraryFixture = "{\"jobs\":[],\"skills\":{\"csharp\":\"mastered\"},\"stacks\":[\"dotnet\"],\"roles\":{\"backend\":true},\"companies\":[\"alpha\"],\"suggestions\":[{\"name\":\"rust\"}],\"future\":{\"retained\":true}}";
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("first", Company("alpha"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
            }
            await ExecuteAsync(path, $"insert into analytics_state(id,filters_json,library_json) values(1,'{{\"countries\":[\"NL\"]}}','{libraryFixture}');");
            await using (var store = await OpenAsync(path))
            {
                await store.ToggleSavedVacancyAsync(new VacancyKey(new CompanyId("alpha"), new SourceId("one")), TestContext.Current.CancellationToken);
                var action = () => store.ToggleSavedVacancyAsync(new VacancyKey(new CompanyId("alpha"), new SourceId("missing")), TestContext.Current.CancellationToken);
                await action.Should().ThrowAsync<KeyNotFoundException>();
                var library = JsonNode.Parse((await store.GetLibraryJsonAsync(TestContext.Current.CancellationToken))!)!.AsObject();
                library["skills"]!["csharp"]!.GetValue<string>().Should().Be("mastered");
                library["stacks"]!.AsArray().Single()!.GetValue<string>().Should().Be("dotnet");
                library["roles"]!["backend"]!.GetValue<bool>().Should().BeTrue();
                library["companies"]!.AsArray().Single()!.GetValue<string>().Should().Be("alpha");
                library["suggestions"]!.AsArray().Should().ContainSingle();
                library["future"]!["retained"]!.GetValue<bool>().Should().BeTrue();
                library["jobs"]!.AsArray().Should().ContainSingle();
            }
            (await ScalarAsync<string>(path, "select filters_json from analytics_state where id=1;")).Should().Be("{\"countries\":[\"NL\"]}");
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-015")]
    public async Task VacancySnapshotEvidence_AfterRestart_ReproducesMeaningfulFeedInputs()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("first", Company("alpha"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("raw", Company("alpha"), [Vacancy("one", rawPayload: "{\"changed\":true}")], At(9), At(10), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("changed", Company("alpha"), [Vacancy("one", "Principal Engineer")], At(10), At(11), TestContext.Current.CancellationToken);
            }

            await using var reopened = await OpenAsync(path);
            var evidence = await reopened.GetVacancySnapshotsAsync(TestContext.Current.CancellationToken);
            evidence.Select(item => item.Title).Should().Equal("Engineer one", "Principal Engineer");
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-014")]
    public async Task GetAllVacanciesAsync_AfterRestart_RehydratesCompleteDetails()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("first", Company("alpha"), [Vacancy("one", "Platform Engineer", "{\"id\":1}")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.ToggleAppliedAsync(new VacancyKey(new CompanyId("alpha"), new SourceId("one")), At(10), TestContext.Current.CancellationToken);
            }

            await using var reopened = await OpenAsync(path);
            var record = (await reopened.GetAllVacanciesAsync(TestContext.Current.CancellationToken)).Single();
            record.Key.Should().Be(new VacancyKey(new CompanyId("alpha"), new SourceId("one")));
            record.Classified.Observed.Title.Should().Be("Platform Engineer");
            record.Classified.Observed.Locations.Should().Equal("Amsterdam");
            record.Classified.Observed.RawPayload.Should().Be("{\"id\":1}");
            record.AppliedAt.Should().Be(At(10));
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-013")]
    public async Task VacancyLifecycle_RawChurnDoesNotSnapshotAndAppliedStateSurvivesReopen()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("first", Company("alpha"), [Vacancy("one", rawPayload: "{\"page\":1}")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.ToggleAppliedAsync(new VacancyKey(new CompanyId("alpha"), new SourceId("one")), At(10), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("raw", Company("alpha"), [Vacancy("one", rawPayload: "{\"page\":2}")], At(10), At(11), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("closed", Company("alpha"), [], At(11), At(12), TestContext.Current.CancellationToken);
            }

            await using (var reopened = await OpenAsync(path))
                await reopened.PersistCompleteScanAsync("reopened", Company("alpha"), [Vacancy("one", rawPayload: "{\"page\":3}")], At(12), At(13), TestContext.Current.CancellationToken);

            (await ScalarAsync<long>(path, "select count(*) from job_snapshots where company_id='alpha' and source_id='one';")).Should().Be(1);
            (await ScalarAsync<string>(path, "select raw_payload from jobs where company_id='alpha' and source_id='one';")).Should().Be("{\"page\":3}");
            (await ScalarAsync<long>(path, "select source_open from jobs where company_id='alpha' and source_id='one';")).Should().Be(1);
            (await ScalarAsync<long>(path, "select is_new from jobs where company_id='alpha' and source_id='one';")).Should().Be(0);
            (await ScalarAsync<string>(path, "select first_seen_at from jobs where company_id='alpha' and source_id='one';")).Should().Be("2026-08-11T09:00:00+00:00");
            (await ScalarAsync<string>(path, "select last_seen_at from jobs where company_id='alpha' and source_id='one';")).Should().Be("2026-08-11T13:00:00+00:00");
            (await ScalarAsync<string>(path, "select reopened_at from jobs where company_id='alpha' and source_id='one';")).Should().Be("2026-08-11T13:00:00+00:00");
            (await ScalarAsync<string>(path, "select applied_at from jobs where company_id='alpha' and source_id='one';")).Should().Be("2026-08-11T10:00:00+00:00");
        }
        finally { File.Delete(path); }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-013")]
    public async Task PersistCompleteScanAsync_MeaningfulChange_CreatesEvidenceSnapshot()
    {
        var path = TemporaryPath();
        try
        {
            await using (var store = await OpenAsync(path))
            {
                await store.SynchronizeCompaniesAsync([Company("alpha")], TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("first", Company("alpha"), [Vacancy("one")], At(8), At(9), TestContext.Current.CancellationToken);
                await store.PersistCompleteScanAsync("changed", Company("alpha"), [Vacancy("one", "Principal Engineer")], At(10), At(11), TestContext.Current.CancellationToken);
            }

            (await ScalarAsync<long>(path, "select count(*) from job_snapshots where company_id='alpha' and source_id='one';")).Should().Be(2);
        }
        finally { File.Delete(path); }
    }

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

    private static ClassifiedVacancy Vacancy(string id, string? title = null, string rawPayload = "{}") => new(
        new ObservedVacancy(new SourceId(id), title ?? $"Engineer {id}", null, null, null, ["Amsterdam"], ["NL"], $"https://example.test/jobs/{id}", $"https://example.test/jobs/{id}/apply", "Description", rawPayload, null),
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
