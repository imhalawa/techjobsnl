namespace TechJobsNL.Core.Domain;

/// <summary>Associates an observed Vacancy with its eligibility decision.</summary>
public sealed record ClassifiedVacancy(ObservedVacancy Observed, Eligibility Eligibility);
