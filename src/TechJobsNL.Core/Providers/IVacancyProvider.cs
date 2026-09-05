using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Providers;

/// <summary>Loads normalized vacancy observations from one configured official company source.</summary>
public interface IVacancyProvider
{
    /// <summary>Gets the stable company identity owned by this provider instance.</summary>
    CompanyId CompanyId { get; }

    /// <summary>Scans only when explicitly invoked and observes caller cancellation.</summary>
    Task<ProviderScanResult> ScanAsync(CancellationToken cancellationToken);
}
