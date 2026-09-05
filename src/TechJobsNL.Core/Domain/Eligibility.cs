namespace TechJobsNL.Core.Domain;

/// <summary>States whether a Vacancy meets the configured eligibility rules and why.</summary>
public sealed record Eligibility(bool IsEligible, string Reason);
