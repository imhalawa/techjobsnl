namespace TechJobsNL.Core.Domain;

/// <summary>Retains the current lifecycle facts for a Vacancy.</summary>
public sealed record VacancyRecord(
    VacancyKey Key,
    ClassifiedVacancy Classified,
    bool SourceOpen,
    bool IsNew,
    DateTimeOffset FirstSeenAt,
    DateTimeOffset LastSeenAt,
    DateTimeOffset? ClosedAt,
    DateTimeOffset? ReopenedAt,
    DateTimeOffset? AppliedAt);
