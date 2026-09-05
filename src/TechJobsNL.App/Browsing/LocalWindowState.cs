using System.Collections.Immutable;
using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.App.Browsing;

public sealed record LocalWindowState(bool IsLoading, bool IsFailed, ImmutableArray<RetainedVacancy> Vacancies, string Message)
{
    public bool IsEmpty => !IsLoading && !IsFailed && Vacancies.IsEmpty;
    public bool HasVacancies => !Vacancies.IsEmpty;
    public bool ShowStatusMessage => !HasVacancies && !IsFailed;
}
