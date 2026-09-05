namespace TechJobsNL.Core.Domain;

/// <summary>Preserves the legacy source-level failure event contract.</summary>
public sealed record Failed : ScanEvent
{
    /// <summary>Initializes a legacy source-level failure event.</summary>
    public Failed(CompanyId companyId, SourceErrorKind kind, string diagnostic)
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
