namespace TechJobsNL.Core.Domain;

/// <summary>Retains the current lifecycle facts for a Vacancy.</summary>
public sealed record VacancyRecord
{
    /// <summary>Initializes a Vacancy lifecycle record.</summary>
    public VacancyRecord(
        VacancyKey key,
        ClassifiedVacancy classified,
        bool sourceOpen,
        bool isNew,
        DateTimeOffset firstSeenAt,
        DateTimeOffset lastSeenAt,
        DateTimeOffset? closedAt,
        DateTimeOffset? reopenedAt,
        DateTimeOffset? appliedAt)
    {
        Key = key;
        Classified = classified;
        SourceOpen = sourceOpen;
        IsNew = isNew;
        FirstSeenAt = firstSeenAt;
        LastSeenAt = lastSeenAt;
        ClosedAt = closedAt;
        ReopenedAt = reopenedAt;
        AppliedAt = appliedAt;
    }

    /// <summary>Gets the stable Vacancy identity.</summary>
    public VacancyKey Key { get; }

    /// <summary>Gets the classified source observation.</summary>
    public ClassifiedVacancy Classified { get; }

    /// <summary>Gets whether the source currently reports the Vacancy as open.</summary>
    public bool SourceOpen { get; }

    /// <summary>Gets whether this Vacancy is new.</summary>
    public bool IsNew { get; }

    /// <summary>Gets the first observation instant.</summary>
    public DateTimeOffset FirstSeenAt { get; }

    /// <summary>Gets the most recent observation instant.</summary>
    public DateTimeOffset LastSeenAt { get; }

    /// <summary>Gets the close instant, when closed.</summary>
    public DateTimeOffset? ClosedAt { get; }

    /// <summary>Gets the reopen instant, when reopened.</summary>
    public DateTimeOffset? ReopenedAt { get; }

    /// <summary>Gets the application instant, when applied.</summary>
    public DateTimeOffset? AppliedAt { get; }
}
