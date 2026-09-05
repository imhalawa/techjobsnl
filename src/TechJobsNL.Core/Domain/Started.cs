namespace TechJobsNL.Core.Domain;

/// <summary>Preserves the legacy source-level start event contract.</summary>
public sealed record Started(string CompanyId) : ScanEvent;
