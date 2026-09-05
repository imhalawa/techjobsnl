namespace TechJobsNL.Core.Domain;

/// <summary>Preserves the legacy source-level completion event contract.</summary>
public sealed record Completed : ScanEvent
{
    /// <summary>Initializes a legacy source-level completion event.</summary>
    public Completed(CompanyId companyId, SourceScan sourceScan)
    {
        if (!companyId.IsValid)
        {
            throw new ArgumentException("A valid company identifier is required.", nameof(companyId));
        }

        CompanyId = companyId;
        SourceScan = sourceScan;
    }

    /// <summary>Gets the Company Profile identity.</summary>
    public CompanyId CompanyId { get; }

    /// <summary>Gets the source result.</summary>
    public SourceScan SourceScan { get; }
}
