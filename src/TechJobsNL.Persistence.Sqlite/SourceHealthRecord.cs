using TechJobsNL.Core.Domain;

namespace TechJobsNL.Persistence.Sqlite;

public sealed record SourceHealthRecord(
    CompanyId CompanyId,
    string CompanyName,
    bool IsEnabled,
    DateTimeOffset? LatestAttemptedAt,
    DateTimeOffset? LatestSuccessfulAt,
    SourceHealth Health,
    SourceErrorKind? LatestErrorKind,
    string? Diagnostic);
