namespace TechJobsNL.Core.Domain;

/// <summary>Reports a company scan that produced incomplete observations.</summary>
public sealed record CompanyIncomplete(string CompanyId, string Diagnostic, int ObservedCount) : ScanEvent;
