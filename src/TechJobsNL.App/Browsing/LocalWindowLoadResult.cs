using System.Collections.Immutable;
using TechJobsNL.Core.Vacancies;

namespace TechJobsNL.App.Browsing;

public abstract record LocalWindowLoadResult
{
    private LocalWindowLoadResult() { }

    public sealed record Ready(ImmutableArray<RetainedVacancy> Vacancies) : LocalWindowLoadResult;
    public sealed record Failed(string Message) : LocalWindowLoadResult;
}
