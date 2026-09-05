using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Profiles;

public sealed record VacancyUpdate(VacancyKey Key, string CompanyName, string Title, VacancyUpdateKind Kind, DateTimeOffset OccurredAt, string? ContentHash);
