namespace TechJobsNL.Core.Domain;

/// <summary>Reports the start of a scan run.</summary>
public sealed record RunStarted : ScanEvent
{
    /// <summary>Initializes a scan-run start event.</summary>
    public RunStarted(string runId, int companyCount)
    {
        RunId = runId;
        CompanyCount = companyCount;
    }

    /// <summary>Gets the scan-run identifier.</summary>
    public string RunId { get; }

    /// <summary>Gets the scheduled Company Profile count.</summary>
    public int CompanyCount { get; }
}
