namespace TechJobsNL.Core.Domain;

/// <summary>Classifies a known source failure for recovery, health, and progress reporting.</summary>
public enum SourceErrorKind
{
    Configuration,
    Transport,
    Timeout,
    RateLimit,
    Schema,
    IncompleteResults,
    Browser,
    Storage
}
