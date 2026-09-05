using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using TechJobsNL.Core.Application.Dispatch;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Ports.Persistence;
using TechJobsNL.Core.Vacancies;
using TechJobsNL.Persistence.Sqlite;
using TechJobsNL.Runtime.Configuration;

namespace TechJobsNL.Runtime.Browsing;

/// <summary>Composes local configuration, compatible SQLite reads, and Core browsing without providers.</summary>
public static class LocalBrowsingRuntime
{
    /// <summary>Loads a snapshot off the presentation thread and closes all database handles before returning.</summary>
    public static Task<LocalBrowsingOpenResult> OpenAsync(string configurationPath, CancellationToken cancellationToken) =>
        Task.Run(() => OpenLocalAsync(configurationPath, cancellationToken), cancellationToken);

    private static async Task<LocalBrowsingOpenResult> OpenLocalAsync(
        string configurationPath, CancellationToken cancellationToken)
    {
        AppConfiguration configuration;
        try
        {
            configuration = await TomlConfigurationFile.LoadAsync(configurationPath, cancellationToken).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception)
        {
            // The startup boundary reports a safe failure without copying configuration values into diagnostics.
            return new LocalBrowsingOpenResult.Failed(LocalBrowsingFailureKind.Configuration,
                "The configuration could not be loaded. Check that the file exists and contains valid settings.", null);
        }

        var result = await OpenDatabaseAsync(configurationPath, configuration.DatabasePath, cancellationToken)
            .ConfigureAwait(false);
        if (result is SqliteOpenResult.Recovered recovered)
        {
            return new LocalBrowsingOpenResult.Failed(LocalBrowsingFailureKind.Recovery,
                "The database could not be upgraded. Its original data was restored and a backup was retained.",
                recovered.BackupPath);
        }
        if (result is not SqliteOpenResult.Opened opened)
        {
            return new LocalBrowsingOpenResult.Failed(LocalBrowsingFailureKind.Database,
                "The local database could not be opened. Check its location and file format.", null);
        }
        var store = opened.Store;
        await using (store.ConfigureAwait(false))
        {
            try
            {
                ILocalVacancyReader reader = store;
                var catalog = await reader.ReadVacancyCatalogAsync(cancellationToken).ConfigureAwait(false);
                var builder = new DispatchRegistryBuilder(new BrowsingDispatchLogger(NullLogger.Instance), TimeProvider.System);
                VacancyBrowsing.Register(builder, catalog);
                return new LocalBrowsingOpenResult.Opened(
                    new LocalBrowsingSession(builder.Build().Queries), opened.MigrationApplied, opened.BackupPath);
            }
            catch (OperationCanceledException)
            {
                throw;
            }
            catch (Exception)
            {
                return new LocalBrowsingOpenResult.Failed(LocalBrowsingFailureKind.Data,
                    "The retained vacancies could not be read. The stored data has been preserved.", opened.BackupPath);
            }
        }
    }

    private static async Task<SqliteOpenResult> OpenDatabaseAsync(
        string configurationPath, string configuredDatabasePath, CancellationToken cancellationToken)
    {
        try
        {
            cancellationToken.ThrowIfCancellationRequested();
            var databasePath = ConfigurationPaths.GetDatabasePath(configurationPath, configuredDatabasePath);
            Directory.CreateDirectory(Path.GetDirectoryName(databasePath)
                ?? throw new InvalidDataException("The database path has no directory."));
            return await SqliteDatabase.OpenAsync(databasePath, cancellationToken).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception)
        {
            return new SqliteOpenResult.Failed(configuredDatabasePath, "The database location is unavailable.");
        }
    }

    private sealed class BrowsingDispatchLogger : IDispatchLogger
    {
        private readonly ILogger _logger;

        public BrowsingDispatchLogger(ILogger logger)
        {
            _logger = logger;
        }

        public void Started(string requestType) =>
            _logger.LogDebug(new EventId(1001, "QueryStarted"), "Query {RequestType} started", requestType);

        public void Completed(string requestType, TimeSpan elapsed, bool succeeded) =>
            _logger.LogDebug(new EventId(1002, "QueryCompleted"),
                "Query {RequestType} completed in {Elapsed} with success {Succeeded}", requestType, elapsed, succeeded);
    }
}
