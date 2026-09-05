using System.Collections.Immutable;

namespace TechJobsNL.Core.Domain;

/// <summary>Represents an exhaustive outcome reported by an official vacancy source.</summary>
public abstract record SourceScan
{
    /// <summary>Reports a source result that is safe to use for vacancy lifecycle updates.</summary>
    public sealed record Complete(ImmutableArray<ObservedVacancy> Observations) : SourceScan;

    /// <summary>Reports observations that cannot safely be treated as a complete source result.</summary>
    public sealed record Incomplete(ImmutableArray<ObservedVacancy> Observations, string Diagnostic) : SourceScan;
}
