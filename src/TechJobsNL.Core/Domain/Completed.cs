namespace TechJobsNL.Core.Domain;

/// <summary>Preserves the legacy source-level completion event contract.</summary>
public sealed record Completed(string CompanyId, SourceScan SourceScan) : ScanEvent;
