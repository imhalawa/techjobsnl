namespace TechJobsNL.Core.Domain;

/// <summary>Reports a company scan that completed with a complete source result.</summary>
public sealed record CompanyCompleted : ScanEvent
{
    /// <summary>Initializes a completed company scan event.</summary>
    public CompanyCompleted(CompanyId companyId, int observedCount, int eligibleCount)
    {
        if (!companyId.IsValid)
        {
            throw new ArgumentException("A valid company identifier is required.", nameof(companyId));
        }

        CompanyId = companyId;
        ObservedCount = observedCount;
        EligibleCount = eligibleCount;
    }

    /// <summary>Gets the Company Profile identity.</summary>
    public CompanyId CompanyId { get; }

    /// <summary>Gets the observation count.</summary>
    public int ObservedCount { get; }

    /// <summary>Gets the eligible observation count.</summary>
    public int EligibleCount { get; }
}
