using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Core.Scanning;

public interface IScanPersistence
{
    Task PersistCompleteAsync(string runId, CompanyConfiguration company, IReadOnlyCollection<ClassifiedVacancy> vacancies, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken);
    Task PersistIncompleteAsync(string runId, CompanyId companyId, string diagnostic, int observedCount, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken);
    Task PersistFailedAsync(string runId, CompanyId companyId, ScanFailure failure, DateTimeOffset startedAt, DateTimeOffset completedAt, CancellationToken cancellationToken);
}
