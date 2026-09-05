using TechJobsNL.Core.Domain;

namespace TechJobsNL.Persistence.Sqlite;

public sealed record AppliedToggleResult(VacancyKey Key, bool IsApplied, DateTimeOffset? AppliedAt);
