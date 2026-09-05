using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Operations;

/// <summary>A retained company scan with a display-safe diagnostic and no vacancy payloads.</summary>
public sealed record ScanOperationalView(
    string RunId,
    CompanyId CompanyId,
    string CompanyName,
    DateTimeOffset StartedAt,
    DateTimeOffset CompletedAt,
    OperationalOutcome Outcome,
    int ObservedCount,
    SourceErrorKind? ErrorKind,
    string? Diagnostic);
