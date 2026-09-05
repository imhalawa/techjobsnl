using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Vacancies;

public sealed record CompanyView(CompanyId Id, string Name, bool IsEnabled);
