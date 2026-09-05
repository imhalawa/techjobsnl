using System.Collections.Immutable;

namespace TechJobsNL.Core.Domain;

/// <summary>Represents an exhaustive outcome reported by an official vacancy source.</summary>
public abstract record SourceScan
{
    /// <summary>Reports a source result that is safe to use for vacancy lifecycle updates.</summary>
    public sealed record Complete : SourceScan
    {
        /// <summary>Initializes a complete source result.</summary>
        public Complete(ImmutableArray<ObservedVacancy> observations)
        {
            Observations = observations;
        }

        /// <summary>Gets all observed Vacancies.</summary>
        public ImmutableArray<ObservedVacancy> Observations { get; }
    }

    /// <summary>Reports observations that cannot safely be treated as a complete source result.</summary>
    public sealed record Incomplete : SourceScan
    {
        /// <summary>Initializes an incomplete source result.</summary>
        public Incomplete(ImmutableArray<ObservedVacancy> observations, string diagnostic)
        {
            Observations = observations;
            Diagnostic = diagnostic;
        }

        /// <summary>Gets all observed Vacancies.</summary>
        public ImmutableArray<ObservedVacancy> Observations { get; }

        /// <summary>Gets the incomplete-result diagnostic.</summary>
        public string Diagnostic { get; }
    }
}
