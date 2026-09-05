namespace TechJobsNL.Core.Domain;

/// <summary>Reports final company result counts for a scan run.</summary>
public sealed record RunFinished(string RunId, int Completed, int Failed, int Incomplete) : ScanEvent;
