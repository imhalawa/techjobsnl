namespace TechJobsNL.Core.Domain;

/// <summary>Reports that scanning one Company Profile has begun.</summary>
public sealed record CompanyStarted : ScanEvent
{
    /// <summary>Initializes a company scan-start event.</summary>
    public CompanyStarted(CompanyId companyId)
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
