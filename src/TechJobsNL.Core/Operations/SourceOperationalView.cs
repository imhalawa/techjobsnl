using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Operations;

/// <summary>A retained company's source health, current adapter label, and display-safe diagnostic.</summary>
public sealed record SourceOperationalView(
    CompanyId CompanyId,
    string CompanyName,
    string Adapter,
    bool IsEnabled,
    DateTimeOffset? LatestAttemptedAt,
    DateTimeOffset? LatestSuccessfulAt,
    OperationalHealth Health,
    SourceErrorKind? LatestErrorKind,
    string? Diagnostic);
