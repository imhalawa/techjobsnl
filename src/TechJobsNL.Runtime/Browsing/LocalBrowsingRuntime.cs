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
    /// <summary>Opens the compatible per-user location, installing defaults only on first launch.</summary>
    public static async Task<LocalBrowsingOpenResult> OpenDefaultAsync(CancellationToken cancellationToken)
    {
        try
        {
            var platform = OperatingSystem.IsWindows() ? OperatingSystemKind.Windows
                : OperatingSystem.IsMacOS() ? OperatingSystemKind.MacOs : OperatingSystemKind.Linux;
            var path = ConfigurationPaths.GetConfigurationPath(platform,
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                Environment.GetEnvironmentVariable("XDG_CONFIG_HOME"),
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData));
            return await CreateAndOpenAsync(path, cancellationToken).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception)
        {
            return new LocalBrowsingOpenResult.Failed(LocalBrowsingFailureKind.Configuration,
                "The local settings folder is unavailable. Check that your user profile is accessible.", null);
        }
    }

    /// <summary>Installs compatible defaults without overwriting existing settings, then opens local data.</summary>
    public static async Task<LocalBrowsingOpenResult> CreateAndOpenAsync(string configurationPath, CancellationToken cancellationToken)
    {
        try
        {
            using var stream = typeof(LocalBrowsingRuntime).Assembly.GetManifestResourceStream("TechJobsNL.DefaultConfiguration.toml")
                ?? throw new InvalidOperationException("Shipped configuration is unavailable.");
            using var reader = new StreamReader(stream);
            var defaults = await reader.ReadToEndAsync(cancellationToken).ConfigureAwait(false);
            await TomlConfigurationFile.EnsureCreatedAsync(configurationPath, defaults, cancellationToken).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception)
        {
            return new LocalBrowsingOpenResult.Failed(LocalBrowsingFailureKind.Configuration,
                "The local settings could not be created. Check that the settings folder is writable.", null);
        }
        return await OpenAsync(configurationPath, cancellationToken).ConfigureAwait(false);
    }

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
