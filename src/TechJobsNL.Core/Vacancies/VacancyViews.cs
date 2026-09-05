using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Vacancies;

/// <summary>Composes canonical vacancy membership and direct search without environmental dependencies.</summary>
public static class VacancyViews
{
    public static IReadOnlyList<VacancyRecord> Query(IEnumerable<VacancyRecord> vacancies, IEnumerable<CompanyView> companies, VacancyView view, string search, DateTimeOffset now, int newJobMaximumAgeDays)
    {
        var companyMap = companies.ToDictionary(static company => company.Id, EqualityComparer<CompanyId>.Default);
        var query = search.Trim();
        return vacancies.Where(vacancy => IsMember(vacancy, companyMap, view, now, newJobMaximumAgeDays) && Matches(vacancy, companyMap, query))
            .OrderByDescending(static vacancy => vacancy.LastSeenAt).ThenBy(static vacancy => vacancy.Key.CompanyId.Value, StringComparer.Ordinal).ThenBy(static vacancy => vacancy.Key.SourceId.Value, StringComparer.Ordinal).ToArray();
    }

    public static int ActiveCount(IEnumerable<VacancyRecord> vacancies, IEnumerable<CompanyView> companies) =>
        Query(vacancies, companies, VacancyView.Active, string.Empty, DateTimeOffset.UnixEpoch, 1).Count;

    private static bool IsMember(VacancyRecord vacancy, IReadOnlyDictionary<CompanyId, CompanyView> companies, VacancyView view, DateTimeOffset now, int maximumAgeDays)
    {
        if (view == VacancyView.All) return true;
        if (!companies.TryGetValue(vacancy.Key.CompanyId, out var company) || !company.IsEnabled || !vacancy.Classified.Eligibility.IsEligible) return false;
        return view switch
        {
            VacancyView.Active => vacancy.SourceOpen,
            VacancyView.New => vacancy.SourceOpen && vacancy.IsNew && vacancy.Classified.Observed.PublishedAt is { } published && published <= now && published >= now.AddDays(-maximumAgeDays),
            VacancyView.Applied => vacancy.AppliedAt is not null,
            VacancyView.History => !vacancy.SourceOpen || vacancy.ReopenedAt is not null,
            _ => throw new ArgumentOutOfRangeException(nameof(view))
        };
    }

    private static bool Matches(VacancyRecord vacancy, IReadOnlyDictionary<CompanyId, CompanyView> companies, string search)
    {
        if (search.Length == 0) return true;
        return vacancy.Classified.Observed.Title.Contains(search, StringComparison.OrdinalIgnoreCase) || vacancy.Key.CompanyId.Value.Contains(search, StringComparison.OrdinalIgnoreCase) ||
            companies.TryGetValue(vacancy.Key.CompanyId, out var company) && company.Name.Contains(search, StringComparison.OrdinalIgnoreCase);
    }
}
