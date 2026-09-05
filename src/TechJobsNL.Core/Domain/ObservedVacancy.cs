using System.Collections.Immutable;

namespace TechJobsNL.Core.Domain;

/// <summary>Retains the normalized facts and unmodified serialized JSON evidence observed at an official vacancy source.</summary>
public sealed record ObservedVacancy
{
    /// <summary>Initializes an observed Vacancy.</summary>
    public ObservedVacancy(
        SourceId sourceId,
        string title,
        string? department,
        string? team,
        string? employmentType,
        ImmutableArray<string> locations,
        ImmutableArray<string> countries,
        string jobUrl,
        string applyUrl,
        string description,
        string rawPayload,
        DateTimeOffset? publishedAt)
    {
        if (!sourceId.IsValid)
        {
            throw new ArgumentException("A valid source identifier is required.", nameof(sourceId));
        }

        SourceId = sourceId;
        Title = title;
        Department = department;
        Team = team;
        EmploymentType = employmentType;
        Locations = locations;
        Countries = countries;
        JobUrl = jobUrl;
        ApplyUrl = applyUrl;
        Description = description;
        RawPayload = rawPayload;
        PublishedAt = publishedAt;
    }

    /// <summary>Gets the official source identity.</summary>
    public SourceId SourceId { get; }

    /// <summary>Gets the source title.</summary>
    public string Title { get; }

    /// <summary>Gets the source department, when provided.</summary>
    public string? Department { get; }

    /// <summary>Gets the source team, when provided.</summary>
    public string? Team { get; }

    /// <summary>Gets the source employment type, when provided.</summary>
    public string? EmploymentType { get; }

    /// <summary>Gets the source locations.</summary>
    public ImmutableArray<string> Locations { get; }

    /// <summary>Gets the source countries.</summary>
    public ImmutableArray<string> Countries { get; }

    /// <summary>Gets the trusted vacancy URL.</summary>
    public string JobUrl { get; }

    /// <summary>Gets the trusted application URL.</summary>
    public string ApplyUrl { get; }

    /// <summary>Gets the normalized vacancy description.</summary>
    public string Description { get; }

    /// <summary>Gets unmodified serialized JSON evidence from the source.</summary>
    public string RawPayload { get; }

    /// <summary>Gets the source publication instant, when provided.</summary>
    public DateTimeOffset? PublishedAt { get; }
}
