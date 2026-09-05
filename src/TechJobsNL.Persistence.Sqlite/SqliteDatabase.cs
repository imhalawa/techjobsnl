using Dapper;
using Microsoft.Data.Sqlite;

namespace TechJobsNL.Persistence.Sqlite;

/// <summary>Opens local SQLite state and applies the additive Rust-compatible schema migration.</summary>
public static class SqliteDatabase
{
    private const string MigrationId = "rust-v1";

    /// <summary>Opens a database, enabling foreign keys and applying the migration ledger when necessary.</summary>
    public static async Task<SqliteOpenResult> OpenAsync(string databasePath, CancellationToken cancellationToken)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(databasePath);
        cancellationToken.ThrowIfCancellationRequested();

        var fullPath = Path.GetFullPath(databasePath);
        var existed = File.Exists(fullPath);
        var connection = new SqliteConnection(new SqliteConnectionStringBuilder
        {
            DataSource = fullPath,
            Mode = SqliteOpenMode.ReadWriteCreate,
            ForeignKeys = true,
            Pooling = false
        }.ToString());
        string? backupPath = null;

        try
        {
            await connection.OpenAsync(cancellationToken).ConfigureAwait(false);
            await connection.ExecuteAsync(new CommandDefinition("pragma foreign_keys = on;", cancellationToken: cancellationToken))
                .ConfigureAwait(false);
            var migrationApplied = await HasMigrationAsync(connection, cancellationToken).ConfigureAwait(false);
            if (migrationApplied)
            {
                return new SqliteOpenResult.Opened(new SqliteStore(connection), false, null);
            }

            if (existed)
            {
                backupPath = await CreateBackupAsync(connection, fullPath, cancellationToken).ConfigureAwait(false);
            }

            await ValidateKnownTableTypesAsync(connection, cancellationToken).ConfigureAwait(false);
            await ApplyMigrationAsync(connection, cancellationToken).ConfigureAwait(false);
            return new SqliteOpenResult.Opened(new SqliteStore(connection), true, backupPath);
        }
        catch (OperationCanceledException)
        {
            await connection.DisposeAsync().ConfigureAwait(false);
            throw;
        }
        catch (Exception exception) when (backupPath is not null)
        {
            await connection.DisposeAsync().ConfigureAwait(false);
            SqliteConnection.ClearAllPools();
            File.Copy(backupPath, fullPath, true);
            return new SqliteOpenResult.Recovered(fullPath, backupPath, exception.Message);
        }
        catch (Exception exception)
        {
            await connection.DisposeAsync().ConfigureAwait(false);
            return new SqliteOpenResult.Failed(fullPath, exception.Message);
        }
    }

    private static async Task<bool> HasMigrationAsync(SqliteConnection connection, CancellationToken cancellationToken)
    {
        const string tableSql = """
            select exists(
                select 1
                from sqlite_master
                where type = 'table' and name = 'schema_migrations'
            );
            """;
        var ledgerExists = await connection.ExecuteScalarAsync<long>(new CommandDefinition(
            tableSql,
            cancellationToken: cancellationToken)).ConfigureAwait(false) == 1;
        if (!ledgerExists)
        {
            return false;
        }

        const string migrationSql = "select exists(select 1 from schema_migrations where id = @migrationId);";
        return await connection.ExecuteScalarAsync<long>(new CommandDefinition(
            migrationSql,
            new { migrationId = MigrationId },
            cancellationToken: cancellationToken)).ConfigureAwait(false) == 1;
    }

    private static async Task<string> CreateBackupAsync(
        SqliteConnection connection,
        string databasePath,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var backupPath = GetAvailableBackupPath(databasePath);
        var backupConnection = new SqliteConnection(new SqliteConnectionStringBuilder
        {
            DataSource = backupPath,
            Mode = SqliteOpenMode.ReadWriteCreate,
            Pooling = false
        }.ToString());
        await using (backupConnection.ConfigureAwait(false))
        {
            await backupConnection.OpenAsync(cancellationToken).ConfigureAwait(false);
            connection.BackupDatabase(backupConnection);
        }
        return backupPath;
    }

    private static async Task ValidateKnownTableTypesAsync(SqliteConnection connection, CancellationToken cancellationToken)
    {
        string[] tableNames =
        [
            "companies",
            "scans",
            "jobs",
            "job_snapshots",
            "job_analytics",
            "analytics_discovery",
            "analytics_state",
            "skill_suggestions"
        ];

        foreach (var tableName in tableNames)
        {
            var objectType = await connection.ExecuteScalarAsync<string?>(new CommandDefinition(
                "select type from sqlite_master where name = @tableName;",
                new { tableName },
                cancellationToken: cancellationToken)).ConfigureAwait(false);
            if (objectType is not null && !string.Equals(objectType, "table", StringComparison.Ordinal))
            {
                throw new InvalidOperationException($"Expected SQLite table '{tableName}' but found {objectType}.");
            }
        }
    }

    private static string GetAvailableBackupPath(string databasePath)
    {
        for (var suffix = 0; ; suffix++)
        {
            var candidate = suffix == 0 ? databasePath + ".bak" : databasePath + $".bak.{suffix}";
            if (!File.Exists(candidate))
            {
                return candidate;
            }
        }
    }

    private static async Task ApplyMigrationAsync(SqliteConnection connection, CancellationToken cancellationToken)
    {
        var transaction = await connection.BeginTransactionAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            var command = new CommandDefinition(SchemaSql, transaction: transaction, cancellationToken: cancellationToken);
            await connection.ExecuteAsync(command).ConfigureAwait(false);
            await connection.ExecuteAsync(new CommandDefinition(
                "insert into schema_migrations (id, applied_at) values (@migrationId, @appliedAt);",
                new { migrationId = MigrationId, appliedAt = DateTimeOffset.UtcNow.ToString("O", System.Globalization.CultureInfo.InvariantCulture) },
                transaction,
                cancellationToken: cancellationToken)).ConfigureAwait(false);
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
        }
        finally
        {
            await transaction.DisposeAsync().ConfigureAwait(false);
        }
    }

    private const string SchemaSql = """
        create table if not exists schema_migrations (
            id text primary key,
            applied_at text not null
        );

        create table if not exists companies (
            id text primary key,
            name text not null,
            enabled integer not null check (enabled in (0, 1)),
            latest_attempted_at text,
            latest_successful_at text,
            health text not null default 'unknown',
            latest_error_kind text,
            latest_diagnostic text
        );

        create table if not exists scans (
            id integer primary key,
            run_id text not null,
            company_id text not null references companies(id),
            started_at text not null,
            completed_at text not null,
            outcome text not null,
            observed_count integer not null,
            error_kind text,
            diagnostic text
        );

        create table if not exists jobs (
            company_id text not null references companies(id),
            source_id text not null,
            title text not null,
            department text,
            team text,
            employment_type text,
            locations_json text not null,
            countries_json text not null,
            job_url text not null,
            apply_url text not null,
            description text not null,
            published_at text,
            raw_payload text not null,
            content_hash text not null,
            eligible integer not null check (eligible in (0, 1)),
            eligibility_reason text not null,
            source_open integer not null check (source_open in (0, 1)),
            is_new integer not null check (is_new in (0, 1)),
            first_seen_at text not null,
            last_seen_at text not null,
            closed_at text,
            reopened_at text,
            applied_at text,
            primary key (company_id, source_id)
        );

        create table if not exists job_snapshots (
            company_id text not null references companies(id),
            source_id text not null,
            content_hash text not null,
            captured_at text not null,
            title text not null,
            metadata_json text not null,
            locations_json text not null,
            job_url text not null,
            apply_url text not null,
            description text not null,
            raw_payload text not null,
            primary key (company_id, source_id, content_hash),
            foreign key (company_id, source_id) references jobs(company_id, source_id)
        );

        create table if not exists job_analytics (
            company_id text not null,
            source_id text not null,
            content_hash text not null,
            extractor_version text not null,
            facts_json text not null,
            primary key (company_id, source_id, content_hash, extractor_version),
            foreign key (company_id, source_id, content_hash)
                references job_snapshots(company_id, source_id, content_hash)
        );

        create table if not exists analytics_discovery (
            cache_key text primary key,
            provider text not null,
            result_json text not null
        );

        create table if not exists analytics_state (
            id integer primary key check (id = 1),
            filters_json text not null,
            library_json text not null
        );

        create table if not exists skill_suggestions (
            name text primary key,
            aliases_json text not null,
            evidence_json text not null,
            status text not null check (status in ('pending', 'approved', 'rejected')),
            created_at text not null
        );
        """;
}
