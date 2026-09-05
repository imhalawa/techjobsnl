namespace TechJobsNL.Core.Domain;

/// <summary>Describes a source failure without exposing exception or transport implementation details.</summary>
public sealed record ScanFailure
{
    /// <summary>Initializes a source failure.</summary>
    public ScanFailure(SourceErrorKind kind, string diagnostic, int? httpStatus = null, TimeSpan? retryAfter = null, bool isRetryable = false)
    {
        Kind = kind;
        Diagnostic = diagnostic;
        HttpStatus = httpStatus;
        RetryAfter = retryAfter;
        IsRetryable = isRetryable;
    }

    /// <summary>Gets the failure kind.</summary>
    public SourceErrorKind Kind { get; }

    /// <summary>Gets the diagnostic safe for source health reporting.</summary>
    public string Diagnostic { get; }

    public int? HttpStatus { get; }

    public TimeSpan? RetryAfter { get; }

    public bool IsRetryable { get; }
}
