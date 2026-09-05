namespace TechJobsNL.Core.Eligibility;

/// <summary>Represents deterministic construction of an eligibility classifier.</summary>
public abstract record EligibilityClassifierCreation
{
    private EligibilityClassifierCreation() { }

    public sealed record Ready(EligibilityClassifier Classifier) : EligibilityClassifierCreation;

    public sealed record InvalidPattern(string Kind, string Pattern, string Message) : EligibilityClassifierCreation;
}
