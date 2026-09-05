namespace TechJobsNL.Core.Domain.Configuration;

/// <summary>Retains analytics settings without executing analytics behavior.</summary>
public sealed record AnalyticsConfiguration
{
    /// <summary>Initializes a new instance of the <see cref="AnalyticsConfiguration"/> class.</summary>
    public AnalyticsConfiguration(
        AnalyticsProvider provider,
        int minimumSkillOccurrence,
        int maximumSkills,
        long aiTimeoutSeconds,
        int minimumCooccurrence)
    {
        Provider = provider;
        MinimumSkillOccurrence = minimumSkillOccurrence;
        MaximumSkills = maximumSkills;
        AiTimeoutSeconds = aiTimeoutSeconds;
        MinimumCooccurrence = minimumCooccurrence;
    }

    /// <summary>Gets the default analytics settings.</summary>
    public static AnalyticsConfiguration Default { get; } = new(AnalyticsProvider.Local, 2, 50, 60, 3);

    /// <summary>Gets the retained provider selection.</summary>
    public AnalyticsProvider Provider { get; }

    /// <summary>Gets the minimum skill observation volume.</summary>
    public int MinimumSkillOccurrence { get; }

    /// <summary>Gets the maximum displayed skills.</summary>
    public int MaximumSkills { get; }

    /// <summary>Gets the retained optional-provider timeout.</summary>
    public long AiTimeoutSeconds { get; }

    /// <summary>Gets the minimum technology cooccurrence volume.</summary>
    public int MinimumCooccurrence { get; }
}
