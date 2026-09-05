namespace TechJobsNL.Core.Domain;

/// <summary>Preserves the legacy source-level start event contract.</summary>
public sealed record Started : ScanEvent
{
    /// <summary>Initializes a legacy source-level start event.</summary>
    public Started(CompanyId companyId)
    {
        if (!companyId.IsValid)
        {
            throw new ArgumentException("A valid company identifier is required.", nameof(companyId));
        }

        CompanyId = companyId;
    }

    /// <summary>Gets the Company Profile identity.</summary>
    public CompanyId CompanyId { get; }
}
