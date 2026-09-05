using FluentAssertions;
using Microsoft.Data.Sqlite;
using TechJobsNL.Persistence.Sqlite;

namespace TechJobsNL.Persistence.Sqlite.Tests;

public sealed class SqliteDatabaseMigrationTests
{
    [Fact]
    [Trait("TaskId", "V0.1.0-010")]
    [Trait("Category", "Integration")]
    public async Task OpenAsync_FreshDatabase_CreatesTheRustSchemaWithForeignKeys()
    {
        var path = CreateDatabasePath();
        try
        {
            var result = await SqliteDatabase.OpenAsync(path, CancellationToken.None);
            var opened = result.Should().BeOfType<SqliteOpenResult.Opened>().Which;
            opened.MigrationApplied.Should().BeTrue();
            await opened.Store.DisposeAsync();

            using (var connection = Open(path, foreignKeys: true))
            {
                TableNames(connection).Should().BeEquivalentTo(
                    "analytics_discovery",
                    "analytics_state",
                    "companies",
                    "job_analytics",
                    "job_snapshots",
                    "jobs",
                    "scans",
                    "schema_migrations",
                    "skill_suggestions");
                ColumnNames(connection, "jobs").Should().Equal(
                    "company_id", "source_id", "title", "department", "team", "employment_type", "locations_json",
                    "countries_json", "job_url", "apply_url", "description", "published_at", "raw_payload", "content_hash",
                    "eligible", "eligibility_reason", "source_open", "is_new", "first_seen_at", "last_seen_at", "closed_at",
                    "reopened_at", "applied_at");
                using var foreignKeyCheck = connection.CreateCommand();
                foreignKeyCheck.CommandText = "pragma foreign_key_check;";
                using var violations = foreignKeyCheck.ExecuteReader();
                violations.HasRows.Should().BeFalse();
            }
        }
        finally
        {
            DeleteTemporaryDirectory(path);
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-010")]
    [Trait("Category", "Integration")]
    public async Task OpenAsync_RustFixture_PreservesDeferredAnalyticsRowsAndCreatesBackup()
    {
        var path = CreateDatabasePath();
        try
        {
            CreateRustFixture(path);
            var result = await SqliteDatabase.OpenAsync(path, CancellationToken.None);
            var opened = result.Should().BeOfType<SqliteOpenResult.Opened>().Which;
            opened.MigrationApplied.Should().BeTrue();
            opened.BackupPath.Should().NotBeNull();
            await opened.Store.DisposeAsync();

            using (var connection = Open(path, foreignKeys: true))
            {
                Scalar<string>(connection, "select filters_json from analytics_state where id = 1;").Should().Be("{\"window_days\":90}");
                Scalar<string>(connection, "select result_json from analytics_discovery where cache_key = 'rust-cache';").Should().Be("[]");
                Scalar<string>(connection, "select status from skill_suggestions where name = 'Rust';").Should().Be("approved");
            }
        }
        finally
        {
            DeleteTemporaryDirectory(path);
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-010")]
    [Trait("Category", "Integration")]
    public async Task OpenAsync_RepeatedOpen_DoesNotRepeatTheLedgerMigration()
    {
        var path = CreateDatabasePath();
        try
        {
            var first = (await SqliteDatabase.OpenAsync(path, CancellationToken.None)).Should().BeOfType<SqliteOpenResult.Opened>().Which;
            await first.Store.DisposeAsync();
            var second = (await SqliteDatabase.OpenAsync(path, CancellationToken.None)).Should().BeOfType<SqliteOpenResult.Opened>().Which;
            second.MigrationApplied.Should().BeFalse();
            await second.Store.DisposeAsync();

            using (var connection = Open(path, foreignKeys: true))
            {
                Scalar<long>(connection, "select count(*) from schema_migrations where id = 'rust-v1';").Should().Be(1);
            }
        }
        finally
        {
            DeleteTemporaryDirectory(path);
        }
    }

    [Fact]
    [Trait("TaskId", "V0.1.0-010")]
    [Trait("Category", "Integration")]
    public async Task OpenAsync_IncompatibleSchema_RestoresTheTrustedDatabaseFromBackup()
    {
        var path = CreateDatabasePath();
        try
        {
            using (var connection = Open(path, foreignKeys: true))
            using (var command = connection.CreateCommand())
            {
                command.CommandText = "create view scans as select 1 as value;";
                command.ExecuteNonQuery();
            }

            var result = await SqliteDatabase.OpenAsync(path, CancellationToken.None);
            var recovered = result.Should().BeOfType<SqliteOpenResult.Recovered>().Which;
            File.Exists(recovered.BackupPath).Should().BeTrue();

            using (var restored = Open(path, foreignKeys: true))
            {
                Scalar<string>(restored, "select type from sqlite_master where name = 'scans';").Should().Be("view");
                TableNames(restored).Should().NotContain("schema_migrations");
            }
        }
        finally
        {
            DeleteTemporaryDirectory(path);
        }
    }

    private static SqliteConnection Open(string path, bool foreignKeys)
    {
        var connection = new SqliteConnection(new SqliteConnectionStringBuilder
        {
            DataSource = path,
            Mode = SqliteOpenMode.ReadWriteCreate,
            ForeignKeys = foreignKeys,
            Pooling = false
        }.ToString());
        connection.Open();
        return connection;
    }

    private static IReadOnlyList<string> TableNames(SqliteConnection connection)
    {
        using var command = connection.CreateCommand();
        command.CommandText = "select name from sqlite_master where type = 'table' order by name;";
        using var reader = command.ExecuteReader();
        var names = new List<string>();
        while (reader.Read())
        {
            names.Add(reader.GetString(0));
        }

        return names;
    }

    private static IReadOnlyList<string> ColumnNames(SqliteConnection connection, string tableName)
    {
        using var command = connection.CreateCommand();
        command.CommandText = $"pragma table_info({tableName});";
        using var reader = command.ExecuteReader();
        var names = new List<string>();
        while (reader.Read())
        {
            names.Add(reader.GetString(1));
        }

        return names;
    }

    private static T Scalar<T>(SqliteConnection connection, string sql)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        return (T)command.ExecuteScalar()!;
    }

    private static string CreateDatabasePath()
    {
        var directory = Path.Combine(Path.GetTempPath(), $"techjobsnl-v010-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        return Path.Combine(directory, "techjobsnl.db");
    }

    private static void DeleteTemporaryDirectory(string databasePath)
    {
        var directory = Path.GetDirectoryName(databasePath);
        if (directory is null || !Directory.Exists(directory))
        {
            return;
        }

        SqliteConnection.ClearAllPools();
        GC.Collect();
        GC.WaitForPendingFinalizers();

        for (var attempt = 0; attempt < 3; attempt++)
        {
            try
            {
                Directory.Delete(directory, true);
                return;
            }
            catch (IOException) when (attempt < 2)
            {
                Thread.Sleep(TimeSpan.FromMilliseconds(50));
            }
            catch (IOException)
            {
                return;
            }
        }
    }

    private static void CreateRustFixture(string path)
    {
        using var connection = Open(path, foreignKeys: true);
        using var command = connection.CreateCommand();
        command.CommandText = RustSchemaSql + """
            insert into analytics_state (id, filters_json, library_json) values (1, '{"window_days":90}', '{}');
            insert into analytics_discovery (cache_key, provider, result_json) values ('rust-cache', 'local', '[]');
            insert into skill_suggestions (name, aliases_json, evidence_json, status, created_at)
            values ('Rust', '["Rust"]', '[]', 'approved', '2026-08-11T10:00:00Z');
            """;
        command.ExecuteNonQuery();
    }

    // Copied from archive/rust/src/storage/schema.rs; intentionally omits only the .NET migration ledger.
    private const string RustSchemaSql = """
        create table companies (id text primary key, name text not null, enabled integer not null check (enabled in (0, 1)), latest_attempted_at text, latest_successful_at text, health text not null default 'unknown', latest_error_kind text, latest_diagnostic text);
        create table scans (id integer primary key, run_id text not null, company_id text not null references companies(id), started_at text not null, completed_at text not null, outcome text not null, observed_count integer not null, error_kind text, diagnostic text);
        create table jobs (company_id text not null references companies(id), source_id text not null, title text not null, department text, team text, employment_type text, locations_json text not null, countries_json text not null, job_url text not null, apply_url text not null, description text not null, published_at text, raw_payload text not null, content_hash text not null, eligible integer not null check (eligible in (0, 1)), eligibility_reason text not null, source_open integer not null check (source_open in (0, 1)), is_new integer not null check (is_new in (0, 1)), first_seen_at text not null, last_seen_at text not null, closed_at text, reopened_at text, applied_at text, primary key (company_id, source_id));
        create table job_snapshots (company_id text not null references companies(id), source_id text not null, content_hash text not null, captured_at text not null, title text not null, metadata_json text not null, locations_json text not null, job_url text not null, apply_url text not null, description text not null, raw_payload text not null, primary key (company_id, source_id, content_hash), foreign key (company_id, source_id) references jobs(company_id, source_id));
        create table job_analytics (company_id text not null, source_id text not null, content_hash text not null, extractor_version text not null, facts_json text not null, primary key (company_id, source_id, content_hash, extractor_version), foreign key (company_id, source_id, content_hash) references job_snapshots(company_id, source_id, content_hash));
        create table analytics_discovery (cache_key text primary key, provider text not null, result_json text not null);
        create table analytics_state (id integer primary key check (id = 1), filters_json text not null, library_json text not null);
        create table skill_suggestions (name text primary key, aliases_json text not null, evidence_json text not null, status text not null check (status in ('pending', 'approved', 'rejected')), created_at text not null);
        """;
}
