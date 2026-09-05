namespace TechJobsNL.Runtime.Browsing;

/// <summary>The result of opening a local browsing session.</summary>
public abstract record LocalBrowsingOpenResult
{
    private LocalBrowsingOpenResult()
    {
    }

    /// <summary>A ready local snapshot and any migration backup made while opening it.</summary>
    public sealed record Opened(LocalBrowsingSession Session, bool MigrationApplied, string? BackupPath)
        : LocalBrowsingOpenResult;

    /// <summary>A startup failure with a user-safe message and any recovery backup.</summary>
    public sealed record Failed(LocalBrowsingFailureKind Kind, string Message, string? BackupPath)
        : LocalBrowsingOpenResult;
}
