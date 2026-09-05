using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Profiles;

public sealed record VacancySnapshotEvidence(VacancyKey Key, string ContentHash, DateTimeOffset CapturedAt, string Title);
