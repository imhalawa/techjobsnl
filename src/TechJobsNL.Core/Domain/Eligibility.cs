namespace TechJobsNL.Core.Domain;

/// <summary>States whether a Vacancy meets the configured eligibility rules and why.</summary>
public sealed record Eligibility
{
    /// <summary>Initializes an eligibility decision.</summary>
    public Eligibility(bool isEligible, string reason)
    {
        IsEligible = isEligible;
        Reason = reason;
    }

    /// <summary>Gets whether the Vacancy is eligible.</summary>
    public bool IsEligible { get; }

    /// <summary>Gets the decision reason.</summary>
    public string Reason { get; }
}
