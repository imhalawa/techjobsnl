using TechJobsNL.Core.Domain;

namespace TechJobsNL.Persistence.Sqlite;

public sealed record ScanHistoryRecord(
    string RunId,
    CompanyId CompanyId,
    string CompanyName,
    DateTimeOffset StartedAt,
    DateTimeOffset CompletedAt,
    SourceHealth Outcome,
    int ObservedCount,
    SourceErrorKind? ErrorKind,
    string? Diagnostic);
