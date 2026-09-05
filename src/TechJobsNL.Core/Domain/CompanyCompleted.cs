namespace TechJobsNL.Core.Domain;

/// <summary>Reports a company scan that completed with a complete source result.</summary>
public sealed record CompanyCompleted(string CompanyId, int ObservedCount, int EligibleCount) : ScanEvent;
