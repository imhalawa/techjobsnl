using TechJobsNL.Core.Domain;

namespace TechJobsNL.Persistence.Sqlite;

public sealed record SavedVacancyToggleResult(VacancyKey Key, bool IsSaved);
