namespace TechJobsNL.Core.Domain;

/// <summary>Identifies a Vacancy at one Company Profile's official source.</summary>
public sealed record VacancyKey(string CompanyId, string SourceId);
