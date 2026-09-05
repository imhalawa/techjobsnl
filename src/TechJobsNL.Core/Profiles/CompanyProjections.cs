using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Profiles;

/// <summary>Builds Company Profiles and chronological updates from durable non-analytics facts.</summary>
public static class CompanyProjections
{
    public static IReadOnlyList<CompanyProfile> Profiles(IEnumerable<CompanyProfileFacts> companies, IEnumerable<VacancyRecord> vacancies)
    {
        var rows = vacancies.ToArray();
        return companies.OrderBy(static company => company.Name, StringComparer.OrdinalIgnoreCase).ThenBy(static company => company.Id.Value, StringComparer.Ordinal)
            .Select(company => new CompanyProfile(company.Id, company.Name, company.IsFollowed, company.OfficialSource, company.Health, company.Diagnostic,
                rows.Where(vacancy => vacancy.Key.CompanyId == company.Id && vacancy.SourceOpen).OrderByDescending(static vacancy => vacancy.LastSeenAt).ToArray())).ToArray();
    }

    public static IReadOnlyList<VacancyUpdate> UpdateFeed(IEnumerable<CompanyProfileFacts> companies, IEnumerable<VacancyRecord> vacancies, IEnumerable<VacancySnapshotEvidence> snapshots)
    {
        var names = companies.ToDictionary(static company => company.Id, static company => company.Name, EqualityComparer<CompanyId>.Default);
        var evidence = snapshots.GroupBy(static snapshot => snapshot.Key).ToDictionary(static group => group.Key, static group => group.OrderBy(static item => item.CapturedAt).ThenBy(static item => item.ContentHash, StringComparer.Ordinal).ToArray());
        var updates = new List<VacancyUpdate>();
        foreach (var vacancy in vacancies)
        {
            var companyName = names.GetValueOrDefault(vacancy.Key.CompanyId, vacancy.Key.CompanyId.Value);
            var title = vacancy.Classified.Observed.Title;
            updates.Add(new VacancyUpdate(vacancy.Key, companyName, title, VacancyUpdateKind.New, vacancy.FirstSeenAt, evidence.GetValueOrDefault(vacancy.Key)?.FirstOrDefault()?.ContentHash));
            if (evidence.TryGetValue(vacancy.Key, out var changes))
                foreach (var changed in changes.Skip(1)) updates.Add(new VacancyUpdate(vacancy.Key, companyName, changed.Title, VacancyUpdateKind.Changed, changed.CapturedAt, changed.ContentHash));
            if (vacancy.ClosedAt is { } closed) updates.Add(new VacancyUpdate(vacancy.Key, companyName, title, VacancyUpdateKind.Closed, closed, null));
            if (vacancy.ReopenedAt is { } reopened) updates.Add(new VacancyUpdate(vacancy.Key, companyName, title, VacancyUpdateKind.Reopened, reopened, null));
        }

        return updates.OrderByDescending(static update => update.OccurredAt).ThenBy(static update => update.Key.CompanyId.Value, StringComparer.Ordinal)
            .ThenBy(static update => update.Key.SourceId.Value, StringComparer.Ordinal).ThenBy(static update => update.Kind).ToArray();
    }
}
