using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.Core.Ports.Persistence;

/// <summary>Loads retained vacancy facts without contacting their sources.</summary>
public interface ILocalVacancyReader
{
    /// <summary>Materializes the local catalogue for browsing.</summary>
    Task<VacancyCatalog> ReadVacancyCatalogAsync(CancellationToken cancellationToken);
}
