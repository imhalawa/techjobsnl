namespace TechJobsNL.Core.Domain;

/// <summary>Reports a company scan that failed without affecting other Company Profiles.</summary>
public sealed record CompanyFailed : ScanEvent
{
    /// <summary>Initializes a failed company scan event.</summary>
    public CompanyFailed(CompanyId companyId, SourceErrorKind kind, string diagnostic)
    {
        if (!companyId.IsValid)
        {
            throw new ArgumentException("A valid company identifier is required.", nameof(companyId));
        }

        CompanyId = companyId;
        Kind = kind;
        Diagnostic = diagnostic;
    }

    /// <summary>Gets the Company Profile identity.</summary>
    public CompanyId CompanyId { get; }

    /// <summary>Gets the failure kind.</summary>
    public SourceErrorKind Kind { get; }

    /// <summary>Gets the diagnostic safe for source health reporting.</summary>
    public string Diagnostic { get; }
}
