using System.Collections.Immutable;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Vacancies;

/// <summary>Local vacancy details for browsing, without raw source payloads.</summary>
public sealed record RetainedVacancy(
    VacancyKey Key,
    string CompanyName,
    string Title,
    ImmutableArray<string> Locations,
    string Description,
    string JobUrl,
    string ApplyUrl,
    bool SourceOpen,
    DateTimeOffset LastSeenAt);
