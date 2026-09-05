namespace TechJobsNL.Core.Domain;

/// <summary>Reports that scanning one Company Profile has begun.</summary>
public sealed record CompanyStarted(string CompanyId) : ScanEvent;
