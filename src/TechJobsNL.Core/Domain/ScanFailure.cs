namespace TechJobsNL.Core.Domain;

/// <summary>Describes a source failure without exposing exception or transport implementation details.</summary>
public sealed record ScanFailure
{
    /// <summary>Initializes a source failure.</summary>
    public ScanFailure(SourceErrorKind kind, string diagnostic)
    {
        Kind = kind;
        Diagnostic = diagnostic;
    }

    /// <summary>Gets the failure kind.</summary>
    public SourceErrorKind Kind { get; }

    /// <summary>Gets the diagnostic safe for source health reporting.</summary>
    public string Diagnostic { get; }
}
