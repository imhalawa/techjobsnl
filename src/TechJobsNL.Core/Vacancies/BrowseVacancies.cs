using System.Collections.Immutable;
using TechJobsNL.Core.Application.Dispatch;

namespace TechJobsNL.Core.Vacancies;

/// <summary>Searches all retained vacancies by title or company, including closed and disabled-company records.</summary>
public sealed record BrowseVacancies(string Search) : IQuery<ImmutableArray<RetainedVacancy>>;
