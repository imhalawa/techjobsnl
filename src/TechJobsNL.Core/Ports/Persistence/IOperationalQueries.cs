using System.Collections.Immutable;
using TechJobsNL.Core.Domain.Configuration;
using TechJobsNL.Core.Operations;

namespace TechJobsNL.Core.Ports.Persistence;

/// <summary>Reads local operational state without contacting vacancy providers.</summary>
public interface IOperationalQueries
{
    /// <summary>Returns the latest 100 company scans, newest completion then newest insertion first.</summary>
    Task<ImmutableArray<ScanOperationalView>> GetRecentScansAsync(CancellationToken cancellationToken);

    /// <summary>Returns every retained company in name order, using current configuration for adapter labels.</summary>
    Task<ImmutableArray<SourceOperationalView>> GetSourcesAsync(
        ImmutableArray<CompanyConfiguration> configuration,
        CancellationToken cancellationToken);
}
