using System.Collections.Immutable;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Operations;
using TechJobsNL.Core.Ports.Persistence;

namespace TechJobsNL.Persistence.Sqlite;

public sealed partial class SqliteStore : IOperationalQueries
{
    /// <inheritdoc />
    public async Task<ImmutableArray<SourceOperationalView>> GetSourcesAsync(
        ImmutableArray<CompanyConfiguration> configuration,
        CancellationToken cancellationToken)
    {
        var sources = await GetSourceHealthAsync(cancellationToken).ConfigureAwait(false);
        var adapters = configuration.ToDictionary(static company => company.Id,
            static company => company.Source.StrategyName, StringComparer.Ordinal);
        return sources.Select(source => new SourceOperationalView(
            source.CompanyId, source.CompanyName, adapters.GetValueOrDefault(source.CompanyId.Value, "Unknown"),
            source.IsEnabled, source.LatestAttemptedAt, source.LatestSuccessfulAt,
            source.Health switch
            {
                SourceHealth.Unknown => OperationalHealth.Unknown,
                SourceHealth.Healthy => OperationalHealth.Healthy,
                SourceHealth.Incomplete => OperationalHealth.Incomplete,
                SourceHealth.Failed => OperationalHealth.Failed,
                _ => throw new InvalidDataException("Unknown source health."),
            },
            source.LatestErrorKind, OperationalRedaction.Redact(source.Diagnostic))).ToImmutableArray();
    }

    /// <inheritdoc />
    public async Task<ImmutableArray<ScanOperationalView>> GetRecentScansAsync(CancellationToken cancellationToken)
    {
        var scans = await GetScanHistoryAsync(cancellationToken).ConfigureAwait(false);
        return scans.Select(static scan => new ScanOperationalView(
            scan.RunId, scan.CompanyId, scan.CompanyName, scan.StartedAt, scan.CompletedAt,
            scan.Outcome switch
            {
                SourceHealth.Healthy => OperationalOutcome.Complete,
                SourceHealth.Incomplete => OperationalOutcome.Incomplete,
                SourceHealth.Failed => OperationalOutcome.Failed,
                _ => throw new InvalidDataException("A scan must have a completed outcome."),
            },
            scan.ObservedCount, scan.ErrorKind, OperationalRedaction.Redact(scan.Diagnostic))).ToImmutableArray();
    }
}
