using System.Collections.Immutable;

namespace TechJobsNL.Core.Eligibility;

/// <summary>Represents either a deterministic eligibility decision or unresolved source location data.</summary>
public abstract record EligibilityClassification
{
    private EligibilityClassification() { }

    public sealed record Decided(TechJobsNL.Core.Domain.Eligibility Eligibility) : EligibilityClassification;

    public sealed record Incomplete(ImmutableArray<string> UnresolvedLocations) : EligibilityClassification;
}
