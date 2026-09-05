namespace TechJobsNL.Core.Domain;

/// <summary>Associates an observed Vacancy with its eligibility decision.</summary>
public sealed record ClassifiedVacancy
{
    /// <summary>Initializes a classified Vacancy.</summary>
    public ClassifiedVacancy(ObservedVacancy observed, Eligibility eligibility)
    {
        Observed = observed;
        Eligibility = eligibility;
    }

    /// <summary>Gets the observed Vacancy.</summary>
    public ObservedVacancy Observed { get; }

    /// <summary>Gets its eligibility decision.</summary>
    public Eligibility Eligibility { get; }
}
