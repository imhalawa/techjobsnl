namespace TechJobsNL.Core.Domain;

/// <summary>Reports a company scan that produced incomplete observations.</summary>
public sealed record CompanyIncomplete : ScanEvent
{
    /// <summary>Initializes an incomplete company scan event.</summary>
    public CompanyIncomplete(CompanyId companyId, string diagnostic, int observedCount)
    {
        if (!companyId.IsValid)
        {
            throw new ArgumentException("A valid company identifier is required.", nameof(companyId));
        }

        CompanyId = companyId;
        Diagnostic = diagnostic;
        ObservedCount = observedCount;
    }

    /// <summary>Gets the Company Profile identity.</summary>
    public CompanyId CompanyId { get; }

    /// <summary>Gets the incomplete-result diagnostic.</summary>
    public string Diagnostic { get; }

    /// <summary>Gets the observation count.</summary>
    public int ObservedCount { get; }
}
