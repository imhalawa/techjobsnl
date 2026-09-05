using System.Collections.Immutable;
using TechJobsNL.Core.Application.Dispatch;

namespace TechJobsNL.Core.Vacancies;

/// <summary>Registers canonical local browsing against an immutable catalogue.</summary>
public static class VacancyBrowsing
{
    /// <summary>Adds the browsing query to explicit application dispatch.</summary>
    public static void Register(DispatchRegistryBuilder builder, VacancyCatalog catalog)
    {
        var handler = new BrowseHandler(catalog);
        builder.RegisterQuery<BrowseVacancies, ImmutableArray<RetainedVacancy>>(handler, handler);
    }

    private sealed class BrowseHandler : IQueryHandler<BrowseVacancies, ImmutableArray<RetainedVacancy>>,
        IRequestValidator<BrowseVacancies>
    {
        private readonly VacancyCatalog _catalog;

        public BrowseHandler(VacancyCatalog catalog)
        {
            _catalog = catalog;
        }

        public Task<ValidationResult> ValidateAsync(BrowseVacancies request, CancellationToken cancellationToken) =>
            Task.FromResult(request.Search is null
                ? new ValidationResult.Invalid("search-required", "Search text must be supplied.")
                : ValidationResult.ValidResult);

        public Task<DispatchResult<ImmutableArray<RetainedVacancy>>> QueryAsync(
            BrowseVacancies query, CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var companies = _catalog.Companies.ToDictionary(static company => company.Id);
            var matches = VacancyViews.Query(_catalog.Vacancies, _catalog.Companies,
                VacancyView.All, query.Search, DateTimeOffset.UnixEpoch, 1);
            var result = matches.Select(vacancy => new RetainedVacancy(
                vacancy.Key, companies[vacancy.Key.CompanyId].Name, vacancy.Classified.Observed.Title,
                vacancy.Classified.Observed.Locations, vacancy.Classified.Observed.Description,
                vacancy.Classified.Observed.JobUrl, vacancy.Classified.Observed.ApplyUrl,
                vacancy.SourceOpen, vacancy.LastSeenAt)).ToImmutableArray();
            cancellationToken.ThrowIfCancellationRequested();
            return Task.FromResult<DispatchResult<ImmutableArray<RetainedVacancy>>>(
                new DispatchResult<ImmutableArray<RetainedVacancy>>.Success(result));
        }
    }
}
