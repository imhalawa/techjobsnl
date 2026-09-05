using System.Collections.Immutable;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Core.Vacancies;

/// <summary>An immutable snapshot of locally retained vacancies and company identities.</summary>
public sealed record VacancyCatalog(ImmutableArray<VacancyRecord> Vacancies, ImmutableArray<CompanyView> Companies);
