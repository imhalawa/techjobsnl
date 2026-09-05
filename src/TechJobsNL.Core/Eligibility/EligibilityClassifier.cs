using System.Collections.Immutable;
using System.Text.RegularExpressions;
using TechJobsNL.Core.Domain;
using TechJobsNL.Core.Domain.Configuration;

namespace TechJobsNL.Core.Eligibility;

/// <summary>Classifies observed vacancies using validated, side-effect-free eligibility rules.</summary>
public sealed class EligibilityClassifier
{
    private readonly ImmutableHashSet<string> countries;
    private readonly ImmutableArray<Regex> includePatterns;
    private readonly ImmutableArray<Regex> excludePatterns;

    private EligibilityClassifier(ImmutableHashSet<string> countries, ImmutableArray<Regex> includePatterns, ImmutableArray<Regex> excludePatterns)
    {
        this.countries = countries;
        this.includePatterns = includePatterns;
        this.excludePatterns = excludePatterns;
    }

    public static EligibilityClassifierCreation Create(FiltersConfiguration filters)
    {
        var invalidInclude = TryCompile("include title", filters.IncludeTitlePatterns, out var include);
        if (invalidInclude is not null) return invalidInclude;
        var invalidExclude = TryCompile("exclude title", filters.ExcludeTitlePatterns, out var exclude);
        if (invalidExclude is not null) return invalidExclude;
        return new EligibilityClassifierCreation.Ready(new EligibilityClassifier(
            filters.Countries.ToImmutableHashSet(StringComparer.Ordinal),
            include,
            exclude));
    }

    public EligibilityClassification Classify(ObservedVacancy vacancy, IReadOnlyDictionary<string, string> locationCountryOverrides)
    {
        IEnumerable<string> resolvedCountries = vacancy.Countries;
        if (vacancy.Countries.IsEmpty)
        {
            var unresolved = vacancy.Locations.Where(location => !locationCountryOverrides.ContainsKey(location)).ToImmutableArray();
            if (!unresolved.IsEmpty) return new EligibilityClassification.Incomplete(unresolved);
            resolvedCountries = vacancy.Locations.Select(location => locationCountryOverrides[location]);
        }

        if (!resolvedCountries.Any(countries.Contains)) return Decision(false, "outside-configured-countries");
        if (excludePatterns.Any(pattern => pattern.IsMatch(vacancy.Title))) return Decision(false, "excluded-title");
        if (!includePatterns.IsEmpty && !includePatterns.Any(pattern => pattern.IsMatch(vacancy.Title))) return Decision(false, "not-included-title");
        return Decision(true, "eligible");
    }

    private static EligibilityClassifierCreation.InvalidPattern? TryCompile(
        string kind,
        ImmutableArray<string> patterns,
        out ImmutableArray<Regex> result)
    {
        var compiled = ImmutableArray.CreateBuilder<Regex>(patterns.Length);
        foreach (var pattern in patterns)
        {
            try { compiled.Add(new Regex(pattern, RegexOptions.IgnoreCase | RegexOptions.CultureInvariant, TimeSpan.FromSeconds(1))); }
            catch (ArgumentException exception)
            {
                result = [];
                return new EligibilityClassifierCreation.InvalidPattern(kind, pattern, exception.Message);
            }
        }

        result = compiled.MoveToImmutable();
        return null;
    }

    private static EligibilityClassification.Decided Decision(bool eligible, string reason) => new(new Domain.Eligibility(eligible, reason));
}
