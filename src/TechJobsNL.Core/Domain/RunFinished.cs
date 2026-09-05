namespace TechJobsNL.Core.Domain;

/// <summary>Reports final company result counts for a scan run.</summary>
public sealed record RunFinished : ScanEvent
{
    /// <summary>Initializes a scan-run completion event.</summary>
    public RunFinished(string runId, int completed, int failed, int incomplete)
    {
        RunId = runId;
        Completed = completed;
        Failed = failed;
        Incomplete = incomplete;
    }

    /// <summary>Gets the scan-run identifier.</summary>
    public string RunId { get; }

    /// <summary>Gets the completed company count.</summary>
    public int Completed { get; }

    /// <summary>Gets the failed company count.</summary>
    public int Failed { get; }

    /// <summary>Gets the incomplete company count.</summary>
    public int Incomplete { get; }
}
