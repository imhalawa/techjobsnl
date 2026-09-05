using System.Collections.Immutable;
using System.Net;
using System.Net.Sockets;
using System.Text.Json;
using FluentAssertions;
using Microsoft.Data.Sqlite;
using TechJobsNL.Core.Application.Dispatch;
using TechJobsNL.Core.Vacancies;
using TechJobsNL.Runtime.Browsing;
using Tomlyn;
using Tomlyn.Model;

namespace TechJobsNL.Runtime.Tests;

[Trait("TaskId", "V0.1.0-076")]
[Trait("Category", "Integration")]
public sealed class LocalBrowsingTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-077")]
    public async Task CreateAndOpenAsync_FreshInstall_CreatesCompatibleDefaultsAndPreservesThemOnReopen()
    {
        var directory = Directory.CreateTempSubdirectory("techjobsnl-first-launch-");
        try
        {
            var path = Path.Combine(directory.FullName, "config.toml");
            var opened = (await LocalBrowsingRuntime.CreateAndOpenAsync(path, Token))
                .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
            await using (var session = opened.Session)
            {
                (await BrowseAsync(session, "")).Should().BeEmpty();
            }
            var customized = (await File.ReadAllTextAsync(path, Token)) + "\n# Retained user choices\n";
            await File.WriteAllTextAsync(path, customized, Token);
            var reopened = (await LocalBrowsingRuntime.CreateAndOpenAsync(path, Token))
                .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
            await reopened.Session.DisposeAsync();
            (await File.ReadAllTextAsync(path, Token)).Should().Be(customized);
        }
        finally
        {
            directory.Delete(true);
        }
    }

    [Fact]
    public async Task OpenAndQuery_CancelledRequests_PreserveDataAndPropagateCancellation()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var before = await File.ReadAllBytesAsync(fixture.DatabasePath, Token);
        using var cancellation = new CancellationTokenSource();
        await cancellation.CancelAsync();
        var start = () => LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, cancellation.Token);

        await start.Should().ThrowAsync<OperationCanceledException>();
        (await File.ReadAllBytesAsync(fixture.DatabasePath, Token)).Should().Equal(before);
        var opened = (await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token))
            .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
        await using var session = opened.Session;
        var query = () => session.Queries.QueryAsync(new BrowseVacancies(""), cancellation.Token);
        await query.Should().ThrowAsync<OperationCanceledException>();
        (await BrowseAsync(session, "")).Should().HaveCount(3);
    }

    [Fact]
    public async Task DisposeAsync_ReadySession_ReleasesHandlesAndRejectsFurtherQueries()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var opened = (await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token))
            .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
        var queries = opened.Session.Queries;
        using (var exclusive = new FileStream(fixture.DatabasePath, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
        {
            exclusive.Length.Should().BeGreaterThan(0);
        }

        await opened.Session.DisposeAsync();
        await opened.Session.DisposeAsync();

        var query = () => queries.QueryAsync(new BrowseVacancies(""), Token);
        await query.Should().ThrowAsync<ObjectDisposedException>();
    }

    [Fact]
    public async Task OpenAsync_FreshDatabaseDirectory_ReturnsHonestEmptyResults()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var configuration = await File.ReadAllTextAsync(fixture.ConfigurationPath, Token);
        await File.WriteAllTextAsync(fixture.ConfigurationPath,
            configuration.Replace("copy.sqlite", ".data/new.sqlite", StringComparison.Ordinal), Token);

        var result = await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token);

        var opened = result.Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
        await using var session = opened.Session;
        (await BrowseAsync(session, "")).Should().BeEmpty();
        (await BrowseAsync(session, "engineer")).Should().BeEmpty();
        opened.BackupPath.Should().BeNull();
    }

    [Fact]
    public async Task OpenAsync_ConfiguredNetworkSource_PreservesAllLocalDataWithoutConnecting()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        try
        {
            var root = TomlSerializer.Deserialize<TomlTable>(await File.ReadAllTextAsync(fixture.ConfigurationPath, Token));
            Assert.NotNull(root);
            var port = ((IPEndPoint)listener.LocalEndpoint).Port;
            root["companies"] = new TomlTableArray
            {
                new TomlTable
                {
                    ["id"] = "alpha", ["name"] = "Configured name", ["enabled"] = true,
                    ["source"] = new TomlTable
                    {
                        ["strategy"] = "recruitee",
                        ["base_url"] = FormattableString.Invariant($"https://127.0.0.1:{port}"),
                    },
                },
            };
            await File.WriteAllTextAsync(fixture.ConfigurationPath, TomlSerializer.Serialize(root), Token);
            var configBefore = await File.ReadAllBytesAsync(fixture.ConfigurationPath, Token);
            var originalBefore = await File.ReadAllBytesAsync(fixture.OriginalPath, Token);
            var dataBefore = await LocalFixture.SnapshotAsync(fixture.DatabasePath);

            var opened = (await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token))
                .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
            await using (var session = opened.Session)
            {
                (await BrowseAsync(session, "")).Should().HaveCount(3);
            }
            var reopened = (await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token))
                .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
            await using (var session = reopened.Session)
            {
                (await BrowseAsync(session, "")).Should().HaveCount(3);
            }

            listener.Pending().Should().BeFalse();
            opened.MigrationApplied.Should().BeTrue();
            var backup = opened.BackupPath;
            Assert.NotNull(backup);
            reopened.MigrationApplied.Should().BeFalse();
            (await LocalFixture.SnapshotAsync(backup)).Should().Equal(dataBefore);
            (await LocalFixture.SnapshotAsync(fixture.DatabasePath)).Should().Equal(dataBefore);
            (await File.ReadAllBytesAsync(fixture.ConfigurationPath, Token)).Should().Equal(configBefore);
            (await File.ReadAllBytesAsync(fixture.OriginalPath, Token)).Should().Equal(originalBefore);
        }
        finally
        {
            listener.Stop();
        }
    }

    [Fact]
    public async Task OpenAsync_InvalidVacancyData_ReportsFailureAndReleasesDatabase()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        await LocalFixture.ExecuteAsync(fixture.DatabasePath, "update jobs set locations_json = 'invalid JSON';");
        var before = await LocalFixture.SnapshotAsync(fixture.DatabasePath);

        var result = await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token);

        result.Should().BeOfType<LocalBrowsingOpenResult.Failed>().Which.Kind
            .Should().Be(LocalBrowsingFailureKind.Data);
        (await LocalFixture.SnapshotAsync(fixture.DatabasePath)).Should().Equal(before);
        using var exclusive = new FileStream(fixture.DatabasePath, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
        exclusive.Length.Should().BeGreaterThan(0);
    }

    [Fact]
    public async Task OpenAsync_IncompatibleSchema_ReportsRecoveryAndPreservesOriginalData()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        await LocalFixture.ExecuteAsync(fixture.DatabasePath, "drop table scans; create view scans as select 1 as retained;");
        var before = await LocalFixture.SnapshotAsync(fixture.DatabasePath);

        var result = await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token);

        var failed = result.Should().BeOfType<LocalBrowsingOpenResult.Failed>().Which;
        failed.Kind.Should().Be(LocalBrowsingFailureKind.Recovery);
        var backup = failed.BackupPath;
        Assert.NotNull(backup);
        (await LocalFixture.SnapshotAsync(fixture.DatabasePath)).Should().Equal(before);
        (await LocalFixture.SnapshotAsync(backup)).Should().Equal(before);
    }

    [Fact]
    public async Task OpenAsync_CorruptDatabase_ReportsFailureWithoutReplacingIt()
    {
        await using var fixture = await LocalFixture.CreateAsync();
        await File.WriteAllTextAsync(fixture.DatabasePath, "not a SQLite database", Token);
        var original = await File.ReadAllBytesAsync(fixture.DatabasePath, Token);

        var result = await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token);

        result.Should().BeOfType<LocalBrowsingOpenResult.Failed>().Which.Kind
            .Should().Be(LocalBrowsingFailureKind.Database);
        (await File.ReadAllBytesAsync(fixture.DatabasePath, Token)).Should().Equal(original);
    }

    [Theory]
    [InlineData("schema_version = [")]
    [InlineData("schema_version = 99")]
    public async Task OpenAsync_InvalidConfiguration_ReportsFailureWithoutChangingFiles(string invalidConfiguration)
    {
        await using var fixture = await LocalFixture.CreateAsync();
        var database = await File.ReadAllBytesAsync(fixture.DatabasePath, Token);
        invalidConfiguration = (await File.ReadAllTextAsync(fixture.ConfigurationPath, Token))
            .Replace("schema_version = 1", invalidConfiguration, StringComparison.Ordinal);
        await File.WriteAllTextAsync(fixture.ConfigurationPath, invalidConfiguration, Token);

        var result = await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token);

        result.Should().BeOfType<LocalBrowsingOpenResult.Failed>().Which.Kind
            .Should().Be(LocalBrowsingFailureKind.Configuration);
        (await File.ReadAllBytesAsync(fixture.DatabasePath, Token)).Should().Equal(database);
        (await File.ReadAllTextAsync(fixture.ConfigurationPath, Token)).Should().Be(invalidConfiguration);
    }

    [Fact]
    public async Task BrowseVacancies_CopiedRustDatabase_SearchesRetainedTitlesAndCompanyNames()
    {
        await using var fixture = await LocalFixture.CreateAsync();

        var opened = (await LocalBrowsingRuntime.OpenAsync(fixture.ConfigurationPath, Token))
            .Should().BeOfType<LocalBrowsingOpenResult.Opened>().Which;
        await using var session = opened.Session;
        var all = await BrowseAsync(session, "");
        var title = await BrowseAsync(session, "  PLATFORM  ");
        var company = await BrowseAsync(session, "beta labs");

        all.Select(static vacancy => vacancy.Key.SourceId.Value).Should().Equal("one", "two", "three");
        title.Select(static vacancy => vacancy.Key.SourceId.Value).Should().Equal("one", "three");
        company.Should().ContainSingle().Which.CompanyName.Should().Be("Beta Labs");
        all[0].Description.Should().Be("Build the platform");
        all[0].Locations.Should().Equal("Amsterdam");
        all[0].JobUrl.Should().Be("https://example.test/jobs/one");
        all[2].SourceOpen.Should().BeFalse();
    }

    private static CancellationToken Token => TestContext.Current.CancellationToken;

    private static async Task<ImmutableArray<RetainedVacancy>> BrowseAsync(LocalBrowsingSession session, string search)
    {
        var result = await session.Queries.QueryAsync(new BrowseVacancies(search), Token).ConfigureAwait(false);
        return result.Should().BeOfType<DispatchResult<ImmutableArray<RetainedVacancy>>.Success>().Which.Value;
    }

    private sealed class LocalFixture : IAsyncDisposable
    {
        private readonly DirectoryInfo _directory;

        private LocalFixture(DirectoryInfo directory)
        {
            _directory = directory;
        }

        public string ConfigurationPath => Path.Combine(_directory.FullName, "config.toml");
        public string DatabasePath => Path.Combine(_directory.FullName, "copy.sqlite");
        public string OriginalPath => Path.Combine(_directory.FullName, "original.sqlite");

        public static async Task<LocalFixture> CreateAsync()
        {
            var fixture = new LocalFixture(Directory.CreateTempSubdirectory("techjobsnl-browsing-"));
            var archive = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "archive", "rust"));
            var configuration = await File.ReadAllTextAsync(Path.Combine(archive, "config.toml"), Token).ConfigureAwait(false);
            await File.WriteAllTextAsync(fixture.ConfigurationPath,
                configuration.Replace(".data/techjobsnl.sqlite3", "copy.sqlite", StringComparison.Ordinal), Token).ConfigureAwait(false);
            var source = await File.ReadAllTextAsync(Path.Combine(archive, "src", "storage", "schema.rs"), Token).ConfigureAwait(false);
            var start = source.IndexOf("r#\"", StringComparison.Ordinal) + 3;
            var end = source.IndexOf("\"#", start, StringComparison.Ordinal);
            await ExecuteAsync(fixture.OriginalPath, source[start..end] + Seed).ConfigureAwait(false);
            File.Copy(fixture.OriginalPath, fixture.DatabasePath);
            return fixture;
        }

        public ValueTask DisposeAsync()
        {
            _directory.Delete(true);
            return ValueTask.CompletedTask;
        }

        public static async Task ExecuteAsync(string path, string sql)
        {
            using var connection = new SqliteConnection($"Data Source={path};Pooling=False");
            await connection.OpenAsync(Token).ConfigureAwait(false);
            using var command = connection.CreateCommand();
            command.CommandText = sql;
            await command.ExecuteNonQueryAsync(Token).ConfigureAwait(false);
        }

        public static async Task<ImmutableArray<string>> SnapshotAsync(string path)
        {
            using var connection = new SqliteConnection($"Data Source={path};Pooling=False;Mode=ReadOnly");
            await connection.OpenAsync(Token).ConfigureAwait(false);
            var snapshot = ImmutableArray.CreateBuilder<string>();
            var names = new List<string>();
            using (var schema = connection.CreateCommand())
            {
                schema.CommandText = "select name, sql from sqlite_master where type in ('table','view') and name <> 'schema_migrations' order by name;";
                using var reader = await schema.ExecuteReaderAsync(Token).ConfigureAwait(false);
                while (await reader.ReadAsync(Token).ConfigureAwait(false))
                {
                    names.Add(reader.GetString(0));
                    snapshot.Add(reader.GetString(1));
                }
            }

            foreach (var name in names)
            {
                using var command = connection.CreateCommand();
                command.CommandText = "select * from \"" + name.Replace("\"", "\"\"", StringComparison.Ordinal) + "\";";
                using var reader = await command.ExecuteReaderAsync(Token).ConfigureAwait(false);
                while (await reader.ReadAsync(Token).ConfigureAwait(false))
                {
                    var values = new object[reader.FieldCount];
                    reader.GetValues(values);
                    snapshot.Add(name + ":" + JsonSerializer.Serialize(values));
                }
            }

            return snapshot.ToImmutable();
        }

        private const string Seed = """
            insert into companies (id, name, enabled) values ('alpha', 'Alpha Engineering', 1), ('beta', 'Beta Labs', 0);
            insert into jobs (company_id, source_id, title, locations_json, countries_json, job_url, apply_url,
                description, raw_payload, content_hash, eligible, eligibility_reason, source_open, is_new, first_seen_at, last_seen_at)
            values ('alpha','one','Platform Engineer','["Amsterdam"]','["NL"]','https://example.test/jobs/one',
                'https://example.test/apply/one','Build the platform','{"retained":"private evidence"}','hash-one',1,'eligible',1,1,
                '2026-08-11T08:00:00+00:00','2026-08-11T10:00:00+00:00'),
                ('beta','two','Backend Engineer','[]','[]','https://example.test/jobs/two','https://example.test/apply/two',
                'Backend services','{}','hash-two',1,'eligible',1,0,'2026-08-11T08:00:00+00:00','2026-08-11T10:00:00+00:00'),
                ('alpha','three','Platform Developer','[]','[]','https://example.test/jobs/three','https://example.test/apply/three',
                'Closed role','{}','hash-three',0,'not eligible',0,0,'2026-08-11T08:00:00+00:00','2026-08-11T09:00:00+00:00');
            insert into analytics_state (id, filters_json, library_json) values (1, '{"window_days":90}', '{"custom":"retained"}');
            insert into analytics_discovery (cache_key, provider, result_json) values ('rust-cache', 'local', '[{"retained":true}]');
            """;
    }
}
