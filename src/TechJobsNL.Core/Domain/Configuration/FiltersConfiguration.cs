using System.Collections.Immutable;

namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Defines retained vacancy eligibility filter settings.</summary>
public sealed record FiltersConfiguration
{
    /// <summary>Initializes a new instance of the <see cref="FiltersConfiguration"/> class.</summary>
    public FiltersConfiguration(
        ImmutableArray<string> countries,
        int newJobMaxAgeDays,
        ImmutableArray<string> includeTitlePatterns,
        ImmutableArray<string> excludeTitlePatterns)
    {
        Countries = countries;
        NewJobMaxAgeDays = newJobMaxAgeDays;
        IncludeTitlePatterns = includeTitlePatterns;
        ExcludeTitlePatterns = excludeTitlePatterns;
    }

    /// <summary>Gets the Rust default for the new vacancy age threshold.</summary>
    public const int DefaultNewJobMaxAgeDays = 7;

    /// <summary>Gets allowed country codes.</summary>
    public ImmutableArray<string> Countries { get; }

    /// <summary>Gets the retained new-vacancy age threshold.</summary>
    public int NewJobMaxAgeDays { get; }

    /// <summary>Gets title patterns that accept a vacancy.</summary>
    public ImmutableArray<string> IncludeTitlePatterns { get; }

    /// <summary>Gets title patterns that reject a vacancy.</summary>
    public ImmutableArray<string> ExcludeTitlePatterns { get; }
}
