namespace TechJobsNL.Core.Domain;

/// <summary>Reports the start of a scan run.</summary>
public sealed record RunStarted(string RunId, int CompanyCount) : ScanEvent;
