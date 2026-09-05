using System.Collections.Immutable;

namespace TechJobsNL.Core.Domain;

/// <summary>Retains the normalized facts and unmodified serialized JSON evidence observed at an official vacancy source.</summary>
public sealed record ObservedVacancy(
    string SourceId,
    string Title,
    string? Department,
    string? Team,
    string? EmploymentType,
    ImmutableArray<string> Locations,
    ImmutableArray<string> Countries,
    string JobUrl,
    string ApplyUrl,
    string Description,
    string RawPayload,
    DateTimeOffset? PublishedAt);
